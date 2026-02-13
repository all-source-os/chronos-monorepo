package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ValidateEventUseCase handles validating an event against a schema via Core.
type ValidateEventUseCase struct {
	coreClient clients.CoreClient
}

// NewValidateEventUseCase creates a new ValidateEventUseCase.
func NewValidateEventUseCase(coreClient clients.CoreClient) *ValidateEventUseCase {
	return &ValidateEventUseCase{coreClient: coreClient}
}

// Execute validates an event against its schema in Core.
func (uc *ValidateEventUseCase) Execute(ctx context.Context, req dto.ValidateEventRequest) (*dto.ValidateEventResponse, error) {
	if uc.coreClient == nil {
		return nil, domain.ErrCoreNotAvailable
	}

	result, err := uc.coreClient.ValidateEvent(ctx, clients.ValidateEventRequest{
		EventType: req.EventType,
		Data:      req.Data,
	})
	if err != nil {
		return nil, err
	}

	return &dto.ValidateEventResponse{
		Valid:  result.Valid,
		Errors: result.Errors,
	}, nil
}
