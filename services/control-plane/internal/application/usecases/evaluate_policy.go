package usecases

import (
	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"sort"
)

// EvaluatePolicyUseCase handles policy evaluation
type EvaluatePolicyUseCase struct {
	policyRepo repositories.PolicyRepository
}

// NewEvaluatePolicyUseCase creates a new EvaluatePolicyUseCase
func NewEvaluatePolicyUseCase(policyRepo repositories.PolicyRepository) *EvaluatePolicyUseCase {
	return &EvaluatePolicyUseCase{
		policyRepo: policyRepo,
	}
}

// Execute evaluates policies for a given resource and attributes
func (uc *EvaluatePolicyUseCase) Execute(req dto.EvaluatePolicyRequest) (*dto.EvaluatePolicyResponse, error) {
	// Get all enabled policies for the resource
	policies, err := uc.policyRepo.FindByResource(req.Resource)
	if err != nil {
		return nil, err
	}

	// Filter to only enabled policies
	enabledPolicies := make([]*entities.Policy, 0)
	for _, p := range policies {
		if p.Enabled {
			enabledPolicies = append(enabledPolicies, p)
		}
	}

	// Sort by priority (higher first)
	sort.Slice(enabledPolicies, func(i, j int) bool {
		return enabledPolicies[i].Priority > enabledPolicies[j].Priority
	})

	// Evaluate policies in priority order
	var reasons []string
	for _, policy := range enabledPolicies {
		matches, err := policy.Evaluate(req.Attributes)
		if err != nil {
			continue
		}

		if matches {
			// Policy matched - return decision
			allowed := policy.Action == entities.ActionAllow

			if policy.Action == entities.ActionWarn {
				reasons = append(reasons, "Policy matched (warn): "+policy.Name)
				continue // Continue evaluating
			}

			return &dto.EvaluatePolicyResponse{
				Allowed:   allowed,
				MatchedID: policy.ID,
				Action:    string(policy.Action),
				Reasons:   append(reasons, "Policy matched: "+policy.Name),
			}, nil
		}
	}

	// No policies matched - default allow
	return &dto.EvaluatePolicyResponse{
		Allowed: true,
		Reasons: append(reasons, "No matching policies, default allow"),
	}, nil
}
