package dto

// CreatePolicyRequest represents a request to create a policy
type CreatePolicyRequest struct {
	ID          string              `json:"id" binding:"required"`
	Name        string              `json:"name" binding:"required"`
	Description string              `json:"description"`
	Resource    string              `json:"resource" binding:"required"`
	Action      string              `json:"action" binding:"required"`
	Conditions  []PolicyConditionDTO `json:"conditions"`
	Priority    int                 `json:"priority"`
}

// PolicyConditionDTO represents a policy condition
type PolicyConditionDTO struct {
	Field    string      `json:"field" binding:"required"`
	Operator string      `json:"operator" binding:"required"`
	Value    interface{} `json:"value" binding:"required"`
}

// UpdatePolicyRequest represents a request to update a policy
type UpdatePolicyRequest struct {
	Name        string              `json:"name"`
	Description string              `json:"description"`
	Conditions  []PolicyConditionDTO `json:"conditions"`
	Priority    int                 `json:"priority"`
	Enabled     *bool               `json:"enabled"`
}

// PolicyResponse represents a policy response
type PolicyResponse struct {
	ID          string              `json:"id"`
	Name        string              `json:"name"`
	Description string              `json:"description"`
	Resource    string              `json:"resource"`
	Action      string              `json:"action"`
	Conditions  []PolicyConditionDTO `json:"conditions"`
	Priority    int                 `json:"priority"`
	Enabled     bool                `json:"enabled"`
}

// EvaluatePolicyRequest represents a request to evaluate policies
type EvaluatePolicyRequest struct {
	Resource   string                 `json:"resource" binding:"required"`
	Attributes map[string]interface{} `json:"attributes" binding:"required"`
}

// EvaluatePolicyResponse represents a policy evaluation response
type EvaluatePolicyResponse struct {
	Allowed   bool     `json:"allowed"`
	MatchedID string   `json:"matched_policy_id,omitempty"`
	Action    string   `json:"action,omitempty"`
	Reasons   []string `json:"reasons,omitempty"`
}
