package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdateConfigUseCase handles updating a config entry.
type UpdateConfigUseCase struct {
	configRepo repositories.ConfigRepository
	auditRepo  repositories.AuditRepository
}

// NewUpdateConfigUseCase creates a new UpdateConfigUseCase.
func NewUpdateConfigUseCase(
	configRepo repositories.ConfigRepository,
	auditRepo repositories.AuditRepository,
) *UpdateConfigUseCase {
	return &UpdateConfigUseCase{
		configRepo: configRepo,
		auditRepo:  auditRepo,
	}
}

// Execute updates an existing config entry.
func (uc *UpdateConfigUseCase) Execute(key string, req dto.UpdateConfigRequest, updatedBy string) (*dto.ConfigResponse, error) {
	entry, err := uc.configRepo.FindByKey(key)
	if err != nil {
		return nil, err
	}

	entry.Update(req.Value, req.Description, updatedBy)

	if err := uc.configRepo.Save(entry); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("config.updated", "update", "PUT", "/config/"+key) //nolint:errcheck
	auditEvent.WithResource("config", key)
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
