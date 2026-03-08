package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListAlertRulesUseCase handles listing all alert rules.
type ListAlertRulesUseCase struct {
	alertRuleRepo repositories.AlertRuleRepository
}

// NewListAlertRulesUseCase creates a new ListAlertRulesUseCase.
func NewListAlertRulesUseCase(alertRuleRepo repositories.AlertRuleRepository) *ListAlertRulesUseCase {
	return &ListAlertRulesUseCase{alertRuleRepo: alertRuleRepo}
}

// Execute lists all alert rules.
func (uc *ListAlertRulesUseCase) Execute() ([]*dto.AlertRuleResponse, error) {
	rules, err := uc.alertRuleRepo.FindAll()
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.AlertRuleResponse, len(rules))
	for i, rule := range rules {
		responses[i] = toAlertRuleResponse(rule)
	}
	return responses, nil
}
