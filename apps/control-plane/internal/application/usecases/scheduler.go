package usecases

import (
	"context"
	"log"
	"sync"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/google/uuid"
)

// ScheduledTask represents a recurring task definition
type ScheduledTask struct {
	Name     string
	Interval time.Duration
	Enabled  bool
}

// OperationScheduler manages background operations (compaction, snapshots, etc.)
type OperationScheduler struct {
	operationRepo repositories.OperationRepository
	auditRepo     repositories.AuditRepository
	coreClient    clients.CoreClient
	tasks         []ScheduledTask
	cancel        context.CancelFunc
	wg            sync.WaitGroup
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
		},
	}
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
