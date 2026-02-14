package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// CancelReplayUseCase cancels a running replay via Core and updates the local operation to canceled.
type CancelReplayUseCase struct {
	opRepo     repositories.OperationRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewCancelReplayUseCase creates a new CancelReplayUseCase.
func NewCancelReplayUseCase(
	opRepo repositories.OperationRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *CancelReplayUseCase {
	return &CancelReplayUseCase{
		opRepo:     opRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute cancels a replay: calls Core to cancel -> updates local operation to canceled -> audit log.
func (uc *CancelReplayUseCase) Execute(ctx context.Context, operationID string) (*entities.Operation, error) {
	op, err := uc.opRepo.FindByID(operationID)
	if err != nil {
		return nil, err
	}

	// Extract replay ID from operation result
	replayID, _ := op.Result["replay_id"].(string) //nolint:errcheck // safe: zero value "" handled below
	if replayID == "" {
		return op, nil
	}

	// Call Core to cancel the replay
	if err := uc.coreClient.CancelReplay(ctx, replayID); err != nil {
		return op, err
	}

	// Update local operation to canceled
	completedAt := time.Now()
	op.Status = entities.OperationCanceled
	op.CompletedAt = &completedAt
	_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("operation.replay_canceled", "cancel_replay", "POST", "/operations/replay/cancel") //nolint:errcheck
	auditEvent.WithResource("operation", op.ID).WithTenant(op.TenantID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return op, nil
}
