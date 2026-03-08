package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdateAlertRuleUseCase handles updating an alert rule.
type UpdateAlertRuleUseCase struct {
	alertRuleRepo repositories.AlertRuleRepository
	auditRepo     repositories.AuditRepository
}

// NewUpdateAlertRuleUseCase creates a new UpdateAlertRuleUseCase.
func NewUpdateAlertRuleUseCase(
	alertRuleRepo repositories.AlertRuleRepository,
	auditRepo repositories.AuditRepository,
) *UpdateAlertRuleUseCase {
	return &UpdateAlertRuleUseCase{
		alertRuleRepo: alertRuleRepo,
		auditRepo:     auditRepo,
	}
}

// Execute updates an existing alert rule.
func (uc *UpdateAlertRuleUseCase) Execute(id string, req dto.UpdateAlertRuleRequest) (*dto.AlertRuleResponse, error) {
	rule, err := uc.alertRuleRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	if err := rule.Update(
		req.Name,
		req.Metric,
		entities.AlertOperator(req.Operator),
		req.Threshold,
		req.Duration,
		req.NotificationChannel,
		req.Enabled,
	); err != nil {
		return nil, err
	}

	if err := uc.alertRuleRepo.Save(rule); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("alert_rule.updated", "update", "PUT", "/admin/alerts/"+id) //nolint:errcheck
	auditEvent.WithResource("alert_rule", rule.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return toAlertRuleResponse(rule), nil
}
