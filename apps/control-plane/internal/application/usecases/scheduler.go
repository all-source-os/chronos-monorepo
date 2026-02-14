package usecases

import (
	"context"
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/usecases/billing"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ScheduledTask represents a recurring task definition
type ScheduledTask struct {
	Name     string
	Interval time.Duration
	Enabled  bool
}

// OperationScheduler manages background operations (compaction, snapshots, etc.)
type OperationScheduler struct {
	operationRepo  repositories.OperationRepository
	auditRepo      repositories.AuditRepository
	coreClient     clients.CoreClient
	reportUsageUC  *billing.ReportUsageUseCase
	tasks          []ScheduledTask
	cancel         context.CancelFunc
	wg             sync.WaitGroup
}

// NewOperationScheduler creates a new OperationScheduler.
func NewOperationScheduler(
	operationRepo repositories.OperationRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *OperationScheduler {
	return &OperationScheduler{
		operationRepo: operationRepo,
		auditRepo:     auditRepo,
		coreClient:    coreClient,
		tasks: []ScheduledTask{
			{Name: "compaction", Interval: 6 * time.Hour, Enabled: true},
			{Name: "overage_reporting", Interval: 1 * time.Hour, Enabled: true},
		},
	}
}

// SetReportUsageUseCase sets the overage reporting use case for the scheduler.
// Must be called before Start if overage_reporting task is enabled.
func (s *OperationScheduler) SetReportUsageUseCase(uc *billing.ReportUsageUseCase) {
	s.reportUsageUC = uc
}

// Start begins running scheduled tasks in the background.
func (s *OperationScheduler) Start(parentCtx context.Context) {
	ctx, cancel := context.WithCancel(parentCtx)
	s.cancel = cancel

	for _, task := range s.tasks {
		if !task.Enabled {
			continue
		}
		s.wg.Add(1)
		go s.runTask(ctx, task)
	}

	log.Printf("Operation scheduler started with %d tasks", len(s.tasks))
}

// Stop cancels all running tasks and waits for them to finish.
func (s *OperationScheduler) Stop() {
	if s.cancel != nil {
		s.cancel()
	}
	s.wg.Wait()
	log.Println("Operation scheduler stopped")
}

func (s *OperationScheduler) runTask(ctx context.Context, task ScheduledTask) {
	defer s.wg.Done()

	ticker := time.NewTicker(task.Interval)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			s.executeTask(ctx, task)
		}
	}
}

func (s *OperationScheduler) executeTask(ctx context.Context, task ScheduledTask) {
	switch task.Name {
	case "compaction":
		s.executeCompaction(ctx)
	case "overage_reporting":
		s.executeOverageReporting(ctx)
	default:
		log.Printf("Unknown scheduled task: %s", task.Name)
	}
}

func (s *OperationScheduler) executeCompaction(ctx context.Context) {
	if s.coreClient == nil {
		return
	}

	now := time.Now()
	op := &entities.Operation{
		ID:          uuid.New().String(),
		Type:        entities.OperationCompaction,
		Status:      entities.OperationPending,
		Parameters:  map[string]any{"scheduled": true},
		InitiatedBy: "scheduler",
		CreatedAt:   now,
	}

	if err := s.operationRepo.Save(op); err != nil {
		log.Printf("Scheduler: failed to save compaction operation: %v", err)
		return
	}

	// Transition to running
	op.Status = entities.OperationRunning
	op.StartedAt = &now
	_ = s.operationRepo.Update(op) //nolint:errcheck

	_, err := s.coreClient.TriggerCompaction(ctx, clients.CompactionRequest{Force: false})
	if err != nil {
		completedAt := time.Now()
		op.Status = entities.OperationFailed
		op.CompletedAt = &completedAt
		op.Error = err.Error()
		log.Printf("Scheduler: compaction failed: %v", err)
	} else {
		completedAt := time.Now()
		op.Status = entities.OperationCompleted
		op.CompletedAt = &completedAt
		op.Result = map[string]any{"triggered_by": "scheduler"}
		log.Println("Scheduler: compaction completed successfully")
	}

	_ = s.operationRepo.Update(op) //nolint:errcheck // best effort update

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("operation.scheduled", "execute", "SCHEDULER", "/compaction") //nolint:errcheck
	auditEvent.WithResource("operation", op.ID)
	_ = s.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical
}

func (s *OperationScheduler) executeOverageReporting(ctx context.Context) {
	if s.reportUsageUC == nil {
		return
	}

	results := s.reportUsageUC.ExecuteAll(ctx)

	var reported, skipped, errored int
	for _, r := range results {
		switch {
		case r.Error != nil:
			errored++
			log.Printf("Scheduler: overage reporting failed for tenant %s: %v", r.TenantID, r.Error)
		case r.Skipped:
			skipped++
		default:
			reported++
		}
	}

	log.Printf("Scheduler: overage reporting complete — reported=%d skipped=%d errors=%d", reported, skipped, errored)

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("billing.overage.scheduled_report", "execute", "SCHEDULER", "/billing/overage") //nolint:errcheck
	auditEvent.AddMetadata("reported", fmt.Sprintf("%d", reported))
	auditEvent.AddMetadata("skipped", fmt.Sprintf("%d", skipped))
	auditEvent.AddMetadata("errors", fmt.Sprintf("%d", errored))
	_ = s.auditRepo.Log(auditEvent) //nolint:errcheck
}
