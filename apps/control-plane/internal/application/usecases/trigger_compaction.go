package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/google/uuid"
)

// TriggerCompactionUseCase orchestrates triggering compaction via Core with local operation tracking.
type TriggerCompactionUseCase struct {
	opRepo     repositories.OperationRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewTriggerCompactionUseCase creates a new TriggerCompactionUseCase.
func NewTriggerCompactionUseCase(
	opRepo repositories.OperationRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *TriggerCompactionUseCase {
	return &TriggerCompactionUseCase{
		opRepo:     opRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute triggers compaction: creates pending Operation -> calls Core -> updates Operation status -> audit log.
func (uc *TriggerCompactionUseCase) Execute(ctx context.Context, force bool, initiatedBy string) (*entities.Operation, error) {
	now := time.Now()
	op := &entities.Operation{
		ID:          uuid.New().String(),
		Type:        entities.OperationCompaction,
		Status:      entities.OperationPending,
		Parameters:  map[string]any{"force": force},
		InitiatedBy: initiatedBy,
		CreatedAt:   now,
	}

	if err := uc.opRepo.Save(op); err != nil {
		return nil, err
	}

	// Transition to running
	op.Status = entities.OperationRunning
	op.StartedAt = &now
	_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update

	// Call Core to trigger compaction
	resp, err := uc.coreClient.TriggerCompaction(ctx, clients.CompactionRequest{
		Force: force,
	})
	if err != nil {
		completedAt := time.Now()
		op.Status = entities.OperationFailed
		op.CompletedAt = &completedAt
		op.Error = err.Error()
		_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update
		return op, err
	}

	// Mark completed
	completedAt := time.Now()
	op.Status = entities.OperationCompleted
	op.CompletedAt = &completedAt
	op.Result = map[string]any{
		"status": resp.Status,
	}
	_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("operation.compaction_triggered", "trigger_compaction", "POST", "/operations/compaction") //nolint:errcheck
	auditEvent.WithResource("operation", op.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return op, nil
}
