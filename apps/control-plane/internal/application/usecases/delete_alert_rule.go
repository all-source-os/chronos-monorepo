package usecases

import (
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// DeleteAlertRuleUseCase handles deleting an alert rule.
type DeleteAlertRuleUseCase struct {
	alertRuleRepo repositories.AlertRuleRepository
	auditRepo     repositories.AuditRepository
}

// NewDeleteAlertRuleUseCase creates a new DeleteAlertRuleUseCase.
func NewDeleteAlertRuleUseCase(
	alertRuleRepo repositories.AlertRuleRepository,
	auditRepo repositories.AuditRepository,
) *DeleteAlertRuleUseCase {
	return &DeleteAlertRuleUseCase{
		alertRuleRepo: alertRuleRepo,
		auditRepo:     auditRepo,
	}
}

// Execute deletes an alert rule by ID.
func (uc *DeleteAlertRuleUseCase) Execute(id string) error {
	if err := uc.alertRuleRepo.Delete(id); err != nil {
		return err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("alert_rule.deleted", "delete", "DELETE", "/admin/alerts/"+id) //nolint:errcheck
	auditEvent.WithResource("alert_rule", id)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return nil
}
