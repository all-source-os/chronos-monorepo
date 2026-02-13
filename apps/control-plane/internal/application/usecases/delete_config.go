package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeleteConfigUseCase handles deleting a config entry.
type DeleteConfigUseCase struct {
	configRepo repositories.ConfigRepository
	auditRepo  repositories.AuditRepository
}

// NewDeleteConfigUseCase creates a new DeleteConfigUseCase.
func NewDeleteConfigUseCase(
	configRepo repositories.ConfigRepository,
	auditRepo repositories.AuditRepository,
) *DeleteConfigUseCase {
	return &DeleteConfigUseCase{
		configRepo: configRepo,
		auditRepo:  auditRepo,
	}
}

// Execute deletes a config entry by key.
func (uc *DeleteConfigUseCase) Execute(key string) error {
	if err := uc.configRepo.Delete(key); err != nil {
		return err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("config.deleted", "delete", "DELETE", "/config/"+key) //nolint:errcheck
	auditEvent.WithResource("config", key)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
