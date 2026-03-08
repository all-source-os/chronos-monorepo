package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListIPRulesUseCase handles listing all IP rules.
type ListIPRulesUseCase struct {
	ipRuleRepo repositories.IPRuleRepository
}

// NewListIPRulesUseCase creates a new ListIPRulesUseCase.
func NewListIPRulesUseCase(ipRuleRepo repositories.IPRuleRepository) *ListIPRulesUseCase {
	return &ListIPRulesUseCase{ipRuleRepo: ipRuleRepo}
}

// Execute lists all IP rules.
func (uc *ListIPRulesUseCase) Execute() ([]*dto.IPRuleResponse, error) {
	rules, err := uc.ipRuleRepo.FindAll()
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.IPRuleResponse, len(rules))
	for i, rule := range rules {
		responses[i] = toIPRuleResponse(rule)
	}
	return responses, nil
}
