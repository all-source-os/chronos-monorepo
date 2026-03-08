package usecases

import (
	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreateAlertRuleUseCase handles creating a new alert rule.
type CreateAlertRuleUseCase struct {
	alertRuleRepo repositories.AlertRuleRepository
	auditRepo     repositories.AuditRepository
}

// NewCreateAlertRuleUseCase creates a new CreateAlertRuleUseCase.
func NewCreateAlertRuleUseCase(
	alertRuleRepo repositories.AlertRuleRepository,
	auditRepo repositories.AuditRepository,
) *CreateAlertRuleUseCase {
	return &CreateAlertRuleUseCase{
		alertRuleRepo: alertRuleRepo,
		auditRepo:     auditRepo,
	}
}

// Execute creates a new alert rule with validation.
func (uc *CreateAlertRuleUseCase) Execute(req dto.CreateAlertRuleRequest) (*dto.AlertRuleResponse, error) {
	id := uuid.New().String()

	exists, _ := uc.alertRuleRepo.Exists(id) //nolint:errcheck
	if exists {
		return nil, domain.ErrAlertRuleAlreadyExists
	}

	rule, err := entities.NewAlertRule(
		id,
		req.Name,
		req.Metric,
		entities.AlertOperator(req.Operator),
		req.Threshold,
		req.Duration,
		req.NotificationChannel,
	)
	if err != nil {
		return nil, err
	}

	if err := uc.alertRuleRepo.Save(rule); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("alert_rule.created", "create", "POST", "/admin/alerts") //nolint:errcheck
	auditEvent.WithResource("alert_rule", rule.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	return toAlertRuleResponse(rule), nil
}

func toAlertRuleResponse(rule *entities.AlertRule) *dto.AlertRuleResponse {
	return &dto.AlertRuleResponse{
		ID:                  rule.ID,
		Name:                rule.Name,
		Metric:              rule.Metric,
		Operator:            string(rule.Operator),
		Threshold:           rule.Threshold,
		Duration:            rule.Duration,
		NotificationChannel: rule.NotificationChannel,
		Enabled:             rule.Enabled,
		CreatedAt:           rule.CreatedAt,
		UpdatedAt:           rule.UpdatedAt,
	}
}
