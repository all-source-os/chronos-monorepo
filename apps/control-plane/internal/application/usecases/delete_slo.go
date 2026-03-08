package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeleteSLOUseCase handles deleting an SLO.
type DeleteSLOUseCase struct {
	sloRepo   repositories.SLORepository
	auditRepo repositories.AuditRepository
}

// NewDeleteSLOUseCase creates a new DeleteSLOUseCase.
func NewDeleteSLOUseCase(
	sloRepo repositories.SLORepository,
	auditRepo repositories.AuditRepository,
) *DeleteSLOUseCase {
	return &DeleteSLOUseCase{
		sloRepo:   sloRepo,
		auditRepo: auditRepo,
	}
}

// Execute deletes an SLO by ID.
func (uc *DeleteSLOUseCase) Execute(id string) error {
	if err := uc.sloRepo.Delete(id); err != nil {
		return err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("slo.deleted", "delete", "DELETE", "/admin/slos/"+id) //nolint:errcheck
	auditEvent.WithResource("slo", id)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
