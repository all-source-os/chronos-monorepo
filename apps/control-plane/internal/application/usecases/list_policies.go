package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ListPoliciesUseCase handles listing policies with optional resource filter.
type ListPoliciesUseCase struct {
	policyRepo repositories.PolicyRepository
}

// NewListPoliciesUseCase creates a new ListPoliciesUseCase.
func NewListPoliciesUseCase(policyRepo repositories.PolicyRepository) *ListPoliciesUseCase {
	return &ListPoliciesUseCase{policyRepo: policyRepo}
}

// Execute retrieves policies, optionally filtered by resource.
func (uc *ListPoliciesUseCase) Execute(resource string) ([]*dto.PolicyResponse, error) {
	var policies []*entities.Policy
	var err error

	if resource != "" {
		policies, err = uc.policyRepo.FindByResource(resource)
	} else {
		policies, err = uc.policyRepo.FindAll()
	}
	if err != nil {
		return nil, err
	}

	responses := make([]*dto.PolicyResponse, len(policies))
	for i, p := range policies {
		conditionDTOs := make([]dto.PolicyConditionDTO, len(p.Conditions))
		for j, c := range p.Conditions {
			conditionDTOs[j] = dto.PolicyConditionDTO{
				Field:    c.Field,
				Operator: c.Operator,
				Value:    c.Value,
			}
		}

		responses[i] = &dto.PolicyResponse{
			ID:          p.ID,
			Name:        p.Name,
			Description: p.Description,
			Resource:    p.Resource,
			Action:      string(p.Action),
			Conditions:  conditionDTOs,
			Priority:    p.Priority,
			Enabled:     p.Enabled,
		}
	}

	return responses, nil
}
