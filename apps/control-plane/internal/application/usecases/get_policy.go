package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// GetPolicyUseCase handles retrieving a single policy by ID.
type GetPolicyUseCase struct {
	policyRepo repositories.PolicyRepository
}

// NewGetPolicyUseCase creates a new GetPolicyUseCase.
func NewGetPolicyUseCase(policyRepo repositories.PolicyRepository) *GetPolicyUseCase {
	return &GetPolicyUseCase{policyRepo: policyRepo}
}

// Execute retrieves a policy by ID and returns a response DTO.
func (uc *GetPolicyUseCase) Execute(id string) (*dto.PolicyResponse, error) {
	policy, err := uc.policyRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	conditionDTOs := make([]dto.PolicyConditionDTO, len(policy.Conditions))
	for i, c := range policy.Conditions {
		conditionDTOs[i] = dto.PolicyConditionDTO{
			Field:    c.Field,
			Operator: c.Operator,
			Value:    c.Value,
		}
	}

	return &dto.PolicyResponse{
		ID:          policy.ID,
		Name:        policy.Name,
		Description: policy.Description,
		Resource:    policy.Resource,
		Action:      string(policy.Action),
		Conditions:  conditionDTOs,
		Priority:    policy.Priority,
		Enabled:     policy.Enabled,
	}, nil
}
