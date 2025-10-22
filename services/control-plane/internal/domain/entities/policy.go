package entities

import (
	"errors"
	"fmt"
)

// PolicyAction represents an action that can be taken
type PolicyAction string

const (
	ActionAllow PolicyAction = "allow"
	ActionDeny  PolicyAction = "deny"
	ActionWarn  PolicyAction = "warn"
)

// PolicyCondition represents a condition for a policy
type PolicyCondition struct {
	Field    string
	Operator string // eq, ne, gt, lt, contains, in
	Value    interface{}
}

// Policy represents a single policy rule
type Policy struct {
	ID          string
	Name        string
	Description string
	Resource    string // tenant, user, operation, etc.
	Action      PolicyAction
	Conditions  []PolicyCondition
	Priority    int  // Higher priority = evaluated first
	Enabled     bool
}

// NewPolicy creates a new policy with validation
func NewPolicy(id, name, description, resource string, action PolicyAction, priority int) (*Policy, error) {
	if err := ValidatePolicyID(id); err != nil {
		return nil, err
	}
	if err := ValidatePolicyName(name); err != nil {
		return nil, err
	}
	if err := ValidatePolicyAction(action); err != nil {
		return nil, err
	}

	return &Policy{
		ID:          id,
		Name:        name,
		Description: description,
		Resource:    resource,
		Action:      action,
		Conditions:  []PolicyCondition{},
		Priority:    priority,
		Enabled:     true,
	}, nil
}

// ValidatePolicyID validates a policy ID
func ValidatePolicyID(id string) error {
	if id == "" {
		return errors.New("policy ID cannot be empty")
	}
	return nil
}

// ValidatePolicyName validates a policy name
func ValidatePolicyName(name string) error {
	if name == "" {
		return errors.New("policy name cannot be empty")
	}
	return nil
}

// ValidatePolicyAction validates a policy action
func ValidatePolicyAction(action PolicyAction) error {
	switch action {
	case ActionAllow, ActionDeny, ActionWarn:
		return nil
	default:
		return errors.New("invalid policy action")
	}
}

// AddCondition adds a condition to the policy
func (p *Policy) AddCondition(field, operator string, value interface{}) error {
	if field == "" {
		return errors.New("condition field cannot be empty")
	}
	if operator == "" {
		return errors.New("condition operator cannot be empty")
	}

	p.Conditions = append(p.Conditions, PolicyCondition{
		Field:    field,
		Operator: operator,
		Value:    value,
	})
	return nil
}

// Enable enables the policy
func (p *Policy) Enable() {
	p.Enabled = true
}

// Disable disables the policy
func (p *Policy) Disable() {
	p.Enabled = false
}

// Evaluate evaluates a policy against given attributes
func (p *Policy) Evaluate(attributes map[string]interface{}) (bool, error) {
	if !p.Enabled {
		return false, nil
	}

	// All conditions must match for policy to apply
	for _, condition := range p.Conditions {
		matches, err := p.evaluateCondition(condition, attributes)
		if err != nil {
			return false, err
		}
		if !matches {
			return false, nil
		}
	}

	return true, nil
}

// evaluateCondition evaluates a single condition
func (p *Policy) evaluateCondition(condition PolicyCondition, attributes map[string]interface{}) (bool, error) {
	attrValue, exists := attributes[condition.Field]
	if !exists {
		return false, nil
	}

	switch condition.Operator {
	case "eq":
		return fmt.Sprintf("%v", attrValue) == fmt.Sprintf("%v", condition.Value), nil
	case "ne":
		return fmt.Sprintf("%v", attrValue) != fmt.Sprintf("%v", condition.Value), nil
	case "contains":
		attrStr := fmt.Sprintf("%v", attrValue)
		valueStr := fmt.Sprintf("%v", condition.Value)
		return contains(attrStr, valueStr), nil
	default:
		return false, fmt.Errorf("unsupported operator: %s", condition.Operator)
	}
}

// contains checks if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && containsHelper(s, substr))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
