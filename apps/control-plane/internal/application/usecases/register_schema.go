package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// RegisterSchemaUseCase handles registering a schema via Core.
type RegisterSchemaUseCase struct {
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient
}

// NewRegisterSchemaUseCase creates a new RegisterSchemaUseCase.
func NewRegisterSchemaUseCase(
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *RegisterSchemaUseCase {
	return &RegisterSchemaUseCase{
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// Execute registers a schema with Core and logs an audit event.
func (uc *RegisterSchemaUseCase) Execute(ctx context.Context, req dto.SchemaGovernanceRequest) (*dto.SchemaGovernanceResponse, error) {
	if uc.coreClient == nil {
		return nil, domain.ErrCoreNotAvailable
	}

	result, err := uc.coreClient.RegisterSchema(ctx, clients.RegisterSchemaRequest{
		EventType: req.EventType,
		Schema:    req.Schema,
		Version:   req.Version,
	})
	if err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("schema.registered", "create", "POST", "/schemas") //nolint:errcheck
	auditEvent.WithResource("schema", req.EventType)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return &dto.SchemaGovernanceResponse{
		EventType: result.EventType,
		Schema:    result.Schema,
		Version:   result.Version,
		Status:    "registered",
	}, nil
}
