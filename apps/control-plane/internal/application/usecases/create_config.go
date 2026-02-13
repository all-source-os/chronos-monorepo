package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreateConfigUseCase handles creating a new config entry.
type CreateConfigUseCase struct {
	configRepo repositories.ConfigRepository
	auditRepo  repositories.AuditRepository
}

// NewCreateConfigUseCase creates a new CreateConfigUseCase.
func NewCreateConfigUseCase(
	configRepo repositories.ConfigRepository,
	auditRepo repositories.AuditRepository,
) *CreateConfigUseCase {
	return &CreateConfigUseCase{
		configRepo: configRepo,
		auditRepo:  auditRepo,
	}
}

// Execute creates a new config entry with validation.
func (uc *CreateConfigUseCase) Execute(req dto.CreateConfigRequest, updatedBy string) (*dto.ConfigResponse, error) {
	entry, err := entities.NewConfigEntry(req.Key, req.Value, req.Description, req.Category)
	if err != nil {
		return nil, err
	}
	entry.UpdatedBy = updatedBy

	if err := uc.configRepo.Save(entry); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("config.created", "create", "POST", "/config") //nolint:errcheck
	auditEvent.WithResource("config", entry.Key)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return &dto.ConfigResponse{
		Key:         entry.Key,
		Value:       entry.Value,
		Description: entry.Description,
		Category:    entry.Category,
		UpdatedBy:   entry.UpdatedBy,
		CreatedAt:   entry.CreatedAt,
		UpdatedAt:   entry.UpdatedAt,
	}, nil
}
