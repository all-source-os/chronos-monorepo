package usecases

import (
	"context"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// GetReplayProgressUseCase polls Core for replay progress and updates the local operation status.
type GetReplayProgressUseCase struct {
	opRepo     repositories.OperationRepository
	coreClient clients.CoreClient
}

// NewGetReplayProgressUseCase creates a new GetReplayProgressUseCase.
func NewGetReplayProgressUseCase(
	opRepo repositories.OperationRepository,
	coreClient clients.CoreClient,
) *GetReplayProgressUseCase {
	return &GetReplayProgressUseCase{
		opRepo:     opRepo,
		coreClient: coreClient,
	}
}

// Execute polls replay progress from Core and updates the local operation.
func (uc *GetReplayProgressUseCase) Execute(ctx context.Context, operationID string) (*entities.Operation, error) {
	op, err := uc.opRepo.FindByID(operationID)
	if err != nil {
		return nil, err
	}

	// Extract replay ID from operation result
	replayID, _ := op.Result["replay_id"].(string)
	if replayID == "" {
		return op, nil
	}

	// Poll Core for progress
	resp, err := uc.coreClient.GetReplayProgress(ctx, replayID)
	if err != nil {
		return op, err
	}

	// Update operation with latest progress
	if op.Result == nil {
		op.Result = make(map[string]any)
	}
	op.Result["progress"] = resp.Progress
	op.Result["status"] = resp.Status

	// If replay completed or failed, update operation status
	switch resp.Status {
	case "completed":
		completedAt := time.Now()
		op.Status = entities.OperationCompleted
		op.CompletedAt = &completedAt
	case "failed":
		completedAt := time.Now()
		op.Status = entities.OperationFailed
		op.CompletedAt = &completedAt
	}

	_ = uc.opRepo.Update(op) //nolint:errcheck // best-effort status update

	return op, nil
}
