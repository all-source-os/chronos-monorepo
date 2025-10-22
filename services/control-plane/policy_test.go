package main

import (
	"testing"

	"github.com/allsource/control-plane/internal/domain/entities"
)

func TestPolicyEngine_Evaluate(t *testing.T) {
	pe := NewPolicyEngine()

	tests := []struct {
		name     string
		ctx      PolicyContext
		expected bool
		action   PolicyAction
	}{
		{
			name: "AdminCanCreateTenant",
			ctx: PolicyContext{
				Resource:  "tenant",
				Operation: "create",
				UserID:    "admin-1",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
			},
			expected: true,
			action:   ActionAllow,
		},
		{
			name: "DeveloperCannotCreateTenant",
			ctx: PolicyContext{
				Resource:  "tenant",
				Operation: "create",
				UserID:    "dev-1",
				TenantID:  "default",
				Role:      entities.RoleDeveloper,
			},
			expected: false,
			action:   ActionDeny,
		},
		{
			name: "CannotDeleteDefaultTenant",
			ctx: PolicyContext{
				Resource:  "tenant",
				Operation: "delete",
				UserID:    "admin-1",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
				Attributes: map[string]interface{}{
					"tenant_id": "default",
				},
			},
			expected: false,
			action:   ActionDeny,
		},
		{
			name: "CanDeleteNonDefaultTenant",
			ctx: PolicyContext{
				Resource:  "tenant",
				Operation: "delete",
				UserID:    "admin-1",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
				Attributes: map[string]interface{}{
					"tenant_id": "custom-tenant",
				},
			},
			expected: true,
			action:   ActionAllow,
		},
		{
			name: "CannotDeleteSelf",
			ctx: PolicyContext{
				Resource:  "user",
				Operation: "delete",
				UserID:    "user-123",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
				Attributes: map[string]interface{}{
					"target_user_id": "user-123",
				},
			},
			expected: false,
			action:   ActionDeny,
		},
		{
			name: "CanDeleteOtherUser",
			ctx: PolicyContext{
				Resource:  "user",
				Operation: "delete",
				UserID:    "user-123",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
				Attributes: map[string]interface{}{
					"target_user_id": "user-456",
				},
			},
			expected: true,
			action:   ActionAllow,
		},
		{
			name: "WarnOnLargeOperation",
			ctx: PolicyContext{
				Resource:  "operation",
				Operation: "bulk_delete",
				UserID:    "admin-1",
				TenantID:  "default",
				Role:      entities.RoleAdmin,
				Attributes: map[string]interface{}{
					"record_count": 15000,
				},
			},
			expected: true, // Allowed but warned
			action:   ActionWarn,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := pe.Evaluate(tt.ctx)

			if result.Allowed != tt.expected {
				t.Errorf("Allowed mismatch: expected %v, got %v (policy: %s, message: %s)",
					tt.expected, result.Allowed, result.PolicyID, result.Message)
			}

			if result.Action != tt.action && result.Action != ActionAllow {
				t.Errorf("Action mismatch: expected %v, got %v", tt.action, result.Action)
			}
		})
	}
}

func TestPolicyEngine_AddRemovePolicy(t *testing.T) {
	pe := NewPolicyEngine()

	// Count initial policies
	initialCount := len(pe.ListPolicies())

	// Add a custom policy
	customPolicy := &Policy{
		ID:          "test-policy",
		Name:        "Test Policy",
		Description: "Test policy for testing",
		Resource:    "test",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "test_field",
				Operator: "eq",
				Value:    "test_value",
			},
		},
		Priority: 999,
		Enabled:  true,
	}

	pe.AddPolicy(customPolicy)

	// Verify policy was added
	policies := pe.ListPolicies()
	if len(policies) != initialCount+1 {
		t.Errorf("Expected %d policies, got %d", initialCount+1, len(policies))
	}

	// Verify we can retrieve it
	retrieved, ok := pe.GetPolicy("test-policy")
	if !ok {
		t.Error("Failed to retrieve added policy")
	}
	if retrieved.Name != "Test Policy" {
		t.Errorf("Policy name mismatch: expected 'Test Policy', got '%s'", retrieved.Name)
	}

	// Remove the policy
	pe.RemovePolicy("test-policy")

	// Verify removal
	policies = pe.ListPolicies()
	if len(policies) != initialCount {
		t.Errorf("Expected %d policies after removal, got %d", initialCount, len(policies))
	}

	// Verify it's gone
	_, ok = pe.GetPolicy("test-policy")
	if ok {
		t.Error("Policy should have been removed")
	}
}

func TestPolicyCondition_Evaluation(t *testing.T) {
	pe := NewPolicyEngine()

	tests := []struct {
		name      string
		condition PolicyCondition
		ctx       PolicyContext
		expected  bool
	}{
		{
			name: "EqualityMatch",
			condition: PolicyCondition{
				Field:    "operation",
				Operator: "eq",
				Value:    "delete",
			},
			ctx: PolicyContext{
				Operation: "delete",
			},
			expected: true,
		},
		{
			name: "EqualityNoMatch",
			condition: PolicyCondition{
				Field:    "operation",
				Operator: "eq",
				Value:    "delete",
			},
			ctx: PolicyContext{
				Operation: "create",
			},
			expected: false,
		},
		{
			name: "NotEqual",
			condition: PolicyCondition{
				Field:    "role",
				Operator: "ne",
				Value:    "Admin",
			},
			ctx: PolicyContext{
				Role: entities.RoleDeveloper,
			},
			expected: true,
		},
		{
			name: "GreaterThan",
			condition: PolicyCondition{
				Field:    "record_count",
				Operator: "gt",
				Value:    10000,
			},
			ctx: PolicyContext{
				Attributes: map[string]interface{}{
					"record_count": 15000,
				},
			},
			expected: true,
		},
		{
			name: "Contains",
			condition: PolicyCondition{
				Field:    "operation",
				Operator: "contains",
				Value:    "delete",
			},
			ctx: PolicyContext{
				Operation: "bulk_delete",
			},
			expected: true,
		},
		{
			name: "InArray",
			condition: PolicyCondition{
				Field:    "operation_type",
				Operator: "in",
				Value:    []string{"snapshot", "backup", "restore"},
			},
			ctx: PolicyContext{
				Attributes: map[string]interface{}{
					"operation_type": "backup",
				},
			},
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := pe.evaluateCondition(tt.condition, tt.ctx)
			if result != tt.expected {
				t.Errorf("Expected %v, got %v", tt.expected, result)
			}
		})
	}
}

func TestExtractResourceAndOperation(t *testing.T) {
	tests := []struct {
		method           string
		path             string
		expectedResource string
		expectedOp       string
	}{
		{"GET", "/api/v1/tenants", "tenant", "read"},
		{"POST", "/api/v1/tenants", "tenant", "create"},
		{"PUT", "/api/v1/tenants/123", "tenant", "update"},
		{"DELETE", "/api/v1/tenants/123", "tenant", "delete"},
		{"GET", "/api/v1/users", "user", "read"},
		{"POST", "/api/v1/operations/snapshot", "operation", "create"},
		{"POST", "/api/v1/auth/login", "auth", "login"},
		{"GET", "/api/v1/metrics", "unknown", "read"},
	}

	for _, tt := range tests {
		t.Run(tt.method+"_"+tt.path, func(t *testing.T) {
			resource, operation := extractResourceAndOperation(tt.method, tt.path)

			if resource != tt.expectedResource {
				t.Errorf("Resource mismatch: expected %s, got %s", tt.expectedResource, resource)
			}

			if operation != tt.expectedOp {
				t.Errorf("Operation mismatch: expected %s, got %s", tt.expectedOp, operation)
			}
		})
	}
}

func TestDefaultPolicies(t *testing.T) {
	pe := NewPolicyEngine()

	// Verify we have default policies
	policies := pe.ListPolicies()
	if len(policies) == 0 {
		t.Error("Expected default policies to be loaded")
	}

	// Verify specific default policies exist
	expectedPolicies := []string{
		"prevent-default-tenant-deletion",
		"require-admin-tenant-create",
		"warn-large-operations",
		"prevent-self-deletion",
		"rate-limit-expensive-ops",
	}

	for _, policyID := range expectedPolicies {
		_, ok := pe.GetPolicy(policyID)
		if !ok {
			t.Errorf("Expected default policy '%s' not found", policyID)
		}
	}
}

func TestPolicyPriority(t *testing.T) {
	pe := NewPolicyEngine()

	// Add two conflicting policies with different priorities
	pe.AddPolicy(&Policy{
		ID:          "high-priority-deny",
		Name:        "High Priority Deny",
		Description: "High priority deny policy",
		Resource:    "test",
		Action:      ActionDeny,
		Conditions: []PolicyCondition{
			{
				Field:    "test_field",
				Operator: "eq",
				Value:    "test",
			},
		},
		Priority: 100,
		Enabled:  true,
	})

	pe.AddPolicy(&Policy{
		ID:          "low-priority-allow",
		Name:        "Low Priority Allow",
		Description: "Low priority allow policy (should be overridden)",
		Resource:    "test",
		Action:      ActionAllow,
		Conditions: []PolicyCondition{
			{
				Field:    "test_field",
				Operator: "eq",
				Value:    "test",
			},
		},
		Priority: 50,
		Enabled:  true,
	})

	// The high priority policy should win
	result := pe.Evaluate(PolicyContext{
		Resource: "test",
		Attributes: map[string]interface{}{
			"test_field": "test",
		},
	})

	if result.Allowed {
		t.Error("High priority deny policy should have taken precedence")
	}

	if result.PolicyID != "high-priority-deny" {
		t.Errorf("Expected policy 'high-priority-deny', got '%s'", result.PolicyID)
	}
}
