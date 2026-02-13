package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ListSchemasUseCase handles listing schemas from Core.
type ListSchemasUseCase struct {
	coreClient clients.CoreClient
}

// NewListSchemasUseCase creates a new ListSchemasUseCase.
func NewListSchemasUseCase(coreClient clients.CoreClient) *ListSchemasUseCase {
	return &ListSchemasUseCase{coreClient: coreClient}
}

// Execute lists all schemas from Core.
func (uc *ListSchemasUseCase) Execute(ctx context.Context) ([]*dto.SchemaGovernanceResponse, error) {
	if uc.coreClient == nil {
		return nil, domain.ErrCoreNotAvailable
	}

	result, err := uc.coreClient.ListSchemas(ctx)
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.SchemaGovernanceResponse, len(result.Schemas))
	for i, schema := range result.Schemas {
		responses[i] = &dto.SchemaGovernanceResponse{
			EventType: schema.EventType,
			Schema:    schema.Schema,
			Version:   schema.Version,
			Status:    "active",
		}
	}
	return responses, nil
}
