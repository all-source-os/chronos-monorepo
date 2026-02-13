package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// CreatePolicyUseCase handles creating a new policy.
type CreatePolicyUseCase struct {
	policyRepo repositories.PolicyRepository
	auditRepo  repositories.AuditRepository
}

// NewCreatePolicyUseCase creates a new CreatePolicyUseCase.
func NewCreatePolicyUseCase(
	policyRepo repositories.PolicyRepository,
	auditRepo repositories.AuditRepository,
) *CreatePolicyUseCase {
	return &CreatePolicyUseCase{
		policyRepo: policyRepo,
		auditRepo:  auditRepo,
	}
}

// Execute validates policy fields, saves to repository, logs audit event, and returns DTO.
func (uc *CreatePolicyUseCase) Execute(req dto.CreatePolicyRequest) (*dto.PolicyResponse, error) {
	// Create domain entity with validation
	policy, err := entities.NewPolicy(
		req.ID,
		req.Name,
		req.Description,
		req.Resource,
		entities.PolicyAction(req.Action),
		req.Priority,
	)
	if err != nil {
		return nil, err
	}

	// Add conditions
	for _, c := range req.Conditions {
		if err := policy.AddCondition(c.Field, c.Operator, c.Value); err != nil {
			return nil, err
		}
	}

	// Persist
	if err := uc.policyRepo.Save(policy); err != nil {
		return nil, err
	}

	// Audit log
	auditEvent, _ := entities.NewAuditEvent("policy.created", "create", "POST", "/policies") //nolint:errcheck
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
