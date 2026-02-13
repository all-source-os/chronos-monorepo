package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// UpdatePolicyUseCase handles updating an existing policy.
type UpdatePolicyUseCase struct {
	policyRepo repositories.PolicyRepository
	auditRepo  repositories.AuditRepository
}

// NewUpdatePolicyUseCase creates a new UpdatePolicyUseCase.
func NewUpdatePolicyUseCase(
	policyRepo repositories.PolicyRepository,
	auditRepo repositories.AuditRepository,
) *UpdatePolicyUseCase {
	return &UpdatePolicyUseCase{
		policyRepo: policyRepo,
		auditRepo:  auditRepo,
	}
}

// Execute validates, finds existing policy, updates fields, saves, and returns DTO.
func (uc *UpdatePolicyUseCase) Execute(id string, req dto.UpdatePolicyRequest) (*dto.PolicyResponse, error) {
	// Find existing policy
	policy, err := uc.policyRepo.FindByID(id)
	if err != nil {
		return nil, err
	}

	// Apply updates
	if req.Name != "" {
		if err := entities.ValidatePolicyName(req.Name); err != nil {
			return nil, err
		}
		policy.Name = req.Name
	}
	if req.Description != "" {
		policy.Description = req.Description
	}
	if req.Conditions != nil {
		conditions := make([]entities.PolicyCondition, len(req.Conditions))
		for i, c := range req.Conditions {
			conditions[i] = entities.PolicyCondition{
				Field:    c.Field,
				Operator: c.Operator,
				Value:    c.Value,
			}
		}
		policy.Conditions = conditions
	}
	if req.Priority != 0 {
		policy.Priority = req.Priority
	}
	if req.Enabled != nil {
		if *req.Enabled {
			policy.Enable()
		} else {
			policy.Disable()
		}
	}

	// Persist
	if err := uc.policyRepo.Update(policy); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("policy.updated", "update", "PUT", "/policies/"+id) //nolint:errcheck
	auditEvent.WithResource("policy", policy.ID)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck // audit logging is non-critical

	// Convert conditions to DTOs
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
