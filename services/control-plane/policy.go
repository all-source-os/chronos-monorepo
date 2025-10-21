package main

import (
	"fmt"
	"strings"
	"sync"

	"github.com/gin-gonic/gin"
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
	Field    string      `json:"field"`
	Operator string      `json:"operator"` // eq, ne, gt, lt, contains, in
	Value    interface{} `json:"value"`
}

// Policy represents a single policy rule
type Policy struct {
	ID          string            `json:"id"`
	Name        string            `json:"name"`
	Description string            `json:"description"`
	Resource    string            `json:"resource"` // tenant, user, operation, etc.
	Action      PolicyAction      `json:"action"`
	Conditions  []PolicyCondition `json:"conditions"`
	Priority    int               `json:"priority"` // Higher priority = evaluated first
	Enabled     bool              `json:"enabled"`
}

// PolicyEngine evaluates policies
type PolicyEngine struct {
	policies map[string]*Policy
	mu       sync.RWMutex
}

// NewPolicyEngine creates a new policy engine
func NewPolicyEngine() *PolicyEngine {
	pe := &PolicyEngine{
		policies: make(map[string]*Policy),
	}

	// Add default policies
	pe.addDefaultPolicies()

	return pe
}

// addDefaultPolicies adds default security policies
func (pe *PolicyEngine) addDefaultPolicies() {
	// Policy 1: Prevent deletion of default tenant
	pe.AddPolicy(&Policy{
		ID:          "prevent-default-tenant-deletion",
		Name:        "Prevent Default Tenant Deletion",
		Description: "Prevents deletion of the default tenant",
		Resource:    "tenant",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "tenant_id",
				Operator: "eq",
				Value:    "default",
			},
			{
				Field:    "operation",
				Operator: "eq",
				Value:    "delete",
			},
		},
		Priority: 100,
		Enabled:  true,
	})

	// Policy 2: Require admin for tenant creation
	pe.AddPolicy(&Policy{
		ID:          "require-admin-tenant-create",
		Name:        "Require Admin for Tenant Creation",
		Description: "Only admins can create new tenants",
		Resource:    "tenant",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "operation",
				Operator: "eq",
				Value:    "create",
			},
			{
				Field:    "role",
				Operator: "ne",
				Value:    "Admin",
			},
		},
		Priority: 90,
		Enabled:  true,
	})

	// Policy 3: Warn on large operations
	pe.AddPolicy(&Policy{
		ID:          "warn-large-operations",
		Name:        "Warn on Large Operations",
		Description: "Warn when operations affect more than 10000 records",
		Resource:    "operation",
		Action:      ActionWarn,
		Conditions: []PolicyCondition{
			{
				Field:    "record_count",
				Operator: "gt",
				Value:    10000,
			},
		},
		Priority: 50,
		Enabled:  true,
	})

	// Policy 4: Prevent self-deletion
	pe.AddPolicy(&Policy{
		ID:          "prevent-self-deletion",
		Name:        "Prevent Self Deletion",
		Description: "Users cannot delete themselves",
		Resource:    "user",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "operation",
				Operator: "eq",
				Value:    "delete",
			},
			{
				Field:    "target_user_id",
				Operator: "eq",
				Value:    "${user_id}", // Special variable
			},
		},
		Priority: 95,
		Enabled:  true,
	})

	// Policy 5: Rate limit expensive operations
	pe.AddPolicy(&Policy{
		ID:          "rate-limit-expensive-ops",
		Name:        "Rate Limit Expensive Operations",
		Description: "Limit snapshot and backup operations",
		Resource:    "operation",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "operation_type",
				Operator: "in",
				Value:    []string{"snapshot", "backup", "restore"},
			},
			{
				Field:    "recent_operations",
				Operator: "gt",
				Value:    5,
			},
		},
		Priority: 80,
		Enabled:  true,
	})
}

// AddPolicy adds a policy to the engine
func (pe *PolicyEngine) AddPolicy(policy *Policy) {
	pe.mu.Lock()
	defer pe.mu.Unlock()

	pe.policies[policy.ID] = policy
}

// RemovePolicy removes a policy from the engine
func (pe *PolicyEngine) RemovePolicy(policyID string) {
	pe.mu.Lock()
	defer pe.mu.Unlock()

	delete(pe.policies, policyID)
}

// GetPolicy retrieves a policy by ID
func (pe *PolicyEngine) GetPolicy(policyID string) (*Policy, bool) {
	pe.mu.RLock()
	defer pe.mu.RUnlock()

	policy, ok := pe.policies[policyID]
	return policy, ok
}

// ListPolicies returns all policies
func (pe *PolicyEngine) ListPolicies() []*Policy {
	pe.mu.RLock()
	defer pe.mu.RUnlock()

	policies := make([]*Policy, 0, len(pe.policies))
	for _, policy := range pe.policies {
		policies = append(policies, policy)
	}

	return policies
}

// PolicyContext holds the context for policy evaluation
type PolicyContext struct {
	Resource   string
	Operation  string
	UserID     string
	TenantID   string
	Role       Role
	Attributes map[string]interface{}
}

// PolicyResult represents the result of policy evaluation
type PolicyResult struct {
	Allowed  bool
	Action   PolicyAction
	PolicyID string
	Message  string
}

// Evaluate evaluates all policies against the given context
func (pe *PolicyEngine) Evaluate(ctx PolicyContext) PolicyResult {
	pe.mu.RLock()
	defer pe.mu.RUnlock()

	// Sort policies by priority (higher first)
	var applicablePolicies []*Policy
	for _, policy := range pe.policies {
		if policy.Enabled && policy.Resource == ctx.Resource {
			applicablePolicies = append(applicablePolicies, policy)
		}
	}

	// Sort by priority
	for i := 0; i < len(applicablePolicies); i++ {
		for j := i + 1; j < len(applicablePolicies); j++ {
			if applicablePolicies[j].Priority > applicablePolicies[i].Priority {
				applicablePolicies[i], applicablePolicies[j] = applicablePolicies[j], applicablePolicies[i]
			}
		}
	}

	// Evaluate policies in priority order
	for _, policy := range applicablePolicies {
		if pe.evaluateConditions(policy.Conditions, ctx) {
			// Policy matched
			if policy.Action == ActionDeny {
				return PolicyResult{
					Allowed:  false,
					Action:   ActionDeny,
					PolicyID: policy.ID,
					Message:  policy.Description,
				}
			} else if policy.Action == ActionWarn {
				// Log warning but continue
				return PolicyResult{
					Allowed:  true,
					Action:   ActionWarn,
					PolicyID: policy.ID,
					Message:  policy.Description,
				}
			}
		}
	}

	// No denying policy matched, allow by default
	return PolicyResult{
		Allowed: true,
		Action:  ActionAllow,
		Message: "No policy matched, default allow",
	}
}

// evaluateConditions checks if all conditions match
func (pe *PolicyEngine) evaluateConditions(conditions []PolicyCondition, ctx PolicyContext) bool {
	for _, condition := range conditions {
		if !pe.evaluateCondition(condition, ctx) {
			return false
		}
	}
	return true
}

// evaluateCondition checks if a single condition matches
func (pe *PolicyEngine) evaluateCondition(condition PolicyCondition, ctx PolicyContext) bool {
	// Get the field value from context
	var fieldValue interface{}
	switch condition.Field {
	case "operation":
		fieldValue = ctx.Operation
	case "user_id":
		fieldValue = ctx.UserID
	case "tenant_id":
		fieldValue = ctx.TenantID
	case "role":
		fieldValue = string(ctx.Role)
	default:
		// Check in attributes
		var ok bool
		fieldValue, ok = ctx.Attributes[condition.Field]
		if !ok {
			return false
		}
	}

	// Handle special variables
	condValue := condition.Value
	if strVal, ok := condValue.(string); ok {
		if strings.HasPrefix(strVal, "${") && strings.HasSuffix(strVal, "}") {
			varName := strings.TrimSuffix(strings.TrimPrefix(strVal, "${"), "}")
			switch varName {
			case "user_id":
				condValue = ctx.UserID
			case "tenant_id":
				condValue = ctx.TenantID
			}
		}
	}

	// Evaluate based on operator
	switch condition.Operator {
	case "eq":
		return fmt.Sprintf("%v", fieldValue) == fmt.Sprintf("%v", condValue)
	case "ne":
		return fmt.Sprintf("%v", fieldValue) != fmt.Sprintf("%v", condValue)
	case "gt":
		if fv, ok := fieldValue.(int); ok {
			if cv, ok := condValue.(int); ok {
				return fv > cv
			}
			if cv, ok := condValue.(float64); ok {
				return fv > int(cv)
			}
		}
		if fv, ok := fieldValue.(float64); ok {
			if cv, ok := condValue.(float64); ok {
				return fv > cv
			}
		}
	case "lt":
		if fv, ok := fieldValue.(int); ok {
			if cv, ok := condValue.(int); ok {
				return fv < cv
			}
		}
	case "contains":
		return strings.Contains(
			fmt.Sprintf("%v", fieldValue),
			fmt.Sprintf("%v", condValue),
		)
	case "in":
		if arr, ok := condValue.([]string); ok {
			fvStr := fmt.Sprintf("%v", fieldValue)
			for _, item := range arr {
				if item == fvStr {
					return true
				}
			}
		}
		// Also handle interface{} arrays
		if arr, ok := condValue.([]interface{}); ok {
			fvStr := fmt.Sprintf("%v", fieldValue)
			for _, item := range arr {
				if fmt.Sprintf("%v", item) == fvStr {
					return true
				}
			}
		}
	}

	return false
}

// PolicyMiddleware enforces policies on requests
func PolicyMiddleware(policyEngine *PolicyEngine, auditLogger *AuditLogger) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Skip health and metrics endpoints
		if c.Request.URL.Path == "/health" || c.Request.URL.Path == "/metrics" {
			c.Next()
			return
		}

		// Extract auth context
		authCtx, err := GetAuthContext(c)
		if err != nil {
			// No auth context, skip policy check (auth middleware will handle)
			c.Next()
			return
		}

		// Determine resource and operation from path
		resource, operation := extractResourceAndOperation(c.Request.Method, c.Request.URL.Path)

		// Build policy context
		policyCtx := PolicyContext{
			Resource:   resource,
			Operation:  operation,
			UserID:     authCtx.UserID,
			TenantID:   authCtx.TenantID,
			Role:       authCtx.Role,
			Attributes: make(map[string]interface{}),
		}

		// Add any additional attributes from request
		if tenantID := c.Param("id"); tenantID != "" && resource == "tenant" {
			policyCtx.Attributes["tenant_id"] = tenantID
		}
		if userID := c.Param("id"); userID != "" && resource == "user" {
			policyCtx.Attributes["target_user_id"] = userID
		}

		// Evaluate policies
		result := policyEngine.Evaluate(policyCtx)

		// Handle result
		if !result.Allowed {
			// Log policy denial
			auditLogger.Log(AuditEvent{
				EventType:  "policy_denial",
				UserID:     authCtx.UserID,
				Username:   authCtx.Username,
				TenantID:   authCtx.TenantID,
				Action:     operation,
				Resource:   resource,
				Method:     c.Request.Method,
				Path:       c.Request.URL.Path,
				StatusCode: 403,
				Error:      result.Message,
				Metadata: map[string]interface{}{
					"policy_id": result.PolicyID,
				},
			})

			c.JSON(403, gin.H{
				"error":     "forbidden",
				"message":   result.Message,
				"policy_id": result.PolicyID,
			})
			c.Abort()
			return
		}

		if result.Action == ActionWarn {
			// Log warning
			auditLogger.Log(AuditEvent{
				EventType: "policy_warning",
				UserID:    authCtx.UserID,
				Username:  authCtx.Username,
				TenantID:  authCtx.TenantID,
				Action:    operation,
				Resource:  resource,
				Method:    c.Request.Method,
				Path:      c.Request.URL.Path,
				Metadata: map[string]interface{}{
					"policy_id": result.PolicyID,
					"warning":   result.Message,
				},
			})
		}

		c.Next()
	}
}

// extractResourceAndOperation extracts resource and operation from HTTP method and path
func extractResourceAndOperation(method, path string) (string, string) {
	resource := "unknown"
	operation := "unknown"

	// Determine resource
	if strings.Contains(path, "/tenants") {
		resource = "tenant"
	} else if strings.Contains(path, "/users") {
		resource = "user"
	} else if strings.Contains(path, "/operations") {
		resource = "operation"
	} else if strings.Contains(path, "/auth") {
		resource = "auth"
	}

	// Determine operation from method
	switch method {
	case "GET":
		operation = "read"
	case "POST":
		if strings.Contains(path, "login") {
			operation = "login"
		} else {
			operation = "create"
		}
	case "PUT":
		operation = "update"
	case "DELETE":
		operation = "delete"
	case "PATCH":
		operation = "modify"
	}

	return resource, operation
}
