package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/google/uuid"
)

// CreateSnapshotUseCase orchestrates creating a snapshot via Core with local operation tracking.
type CreateSnapshotUseCase struct {
	opRepo     repositories.OperationRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewCreateSnapshotUseCase creates a new CreateSnapshotUseCase.
func NewCreateSnapshotUseCase(
	opRepo repositories.OperationRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *CreateSnapshotUseCase {
	return &CreateSnapshotUseCase{
		opRepo:     opRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute creates a snapshot: creates pending Operation -> calls Core -> updates Operation status -> audit log.
func (uc *CreateSnapshotUseCase) Execute(ctx context.Context, tenantID, initiatedBy string) (*entities.Operation, error) {
	now := time.Now()
	op := &entities.Operation{
		ID:          uuid.New().String(),
		Type:        entities.OperationSnapshot,
		Status:      entities.OperationPending,
		TenantID:    tenantID,
		Parameters:  map[string]any{"tenant_id": tenantID},
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

	// Call Core to create snapshot
	resp, err := uc.coreClient.CreateSnapshot(ctx, clients.CreateSnapshotRequest{
		TenantID: tenantID,
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
		"snapshot_id": resp.SnapshotID,
		"status":      resp.Status,
	}
	_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("operation.snapshot_created", "create_snapshot", "POST", "/operations/snapshot") //nolint:errcheck
	auditEvent.WithResource("operation", op.ID).WithTenant(tenantID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return op, nil
}
