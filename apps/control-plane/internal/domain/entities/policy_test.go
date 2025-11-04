package entities

import (
	"testing"
)

func TestNewPolicy(t *testing.T) {
	policy, err := NewPolicy("policy-1", "Test Policy", "Test description", "tenant", ActionDeny, 100)
	if err != nil {
		t.Fatalf("NewPolicy() failed: %v", err)
	}

	if policy.ID != "policy-1" {
		t.Errorf("Policy.ID = %v, want policy-1", policy.ID)
	}
	if policy.Action != ActionDeny {
		t.Errorf("Policy.Action = %v, want %v", policy.Action, ActionDeny)
	}
	if !policy.Enabled {
		t.Error("New policy should be enabled")
	}
}

func TestPolicy_AddCondition(t *testing.T) {
	policy, _ := NewPolicy("policy-1", "Test", "Description", "tenant", ActionDeny, 100)

	err := policy.AddCondition("tenant_id", "eq", "default")
	if err != nil {
		t.Errorf("AddCondition() failed: %v", err)
	}

	if len(policy.Conditions) != 1 {
		t.Errorf("Policy should have 1 condition, got %d", len(policy.Conditions))
	}
}

func TestPolicy_Evaluate(t *testing.T) {
	policy, _ := NewPolicy("policy-1", "Test", "Description", "tenant", ActionDeny, 100)
	_ = policy.AddCondition("tenant_id", "eq", "default")
	_ = policy.AddCondition("operation", "eq", "delete")

	tests := []struct {
		name       string
		attributes map[string]interface{}
		want       bool
	}{
		{
			name: "Both conditions match",
			attributes: map[string]interface{}{
				"tenant_id": "default",
				"operation": "delete",
			},
			want: true,
		},
		{
			name: "Only first condition matches",
			attributes: map[string]interface{}{
				"tenant_id": "default",
				"operation": "create",
			},
			want: false,
		},
		{
			name: "No conditions match",
			attributes: map[string]interface{}{
				"tenant_id": "tenant-1",
				"operation": "create",
			},
			want: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			matches, err := policy.Evaluate(tt.attributes)
			if err != nil {
				t.Errorf("Evaluate() error = %v", err)
				return
			}
			if matches != tt.want {
				t.Errorf("Evaluate() = %v, want %v", matches, tt.want)
			}
		})
	}
}

func TestPolicy_Disabled(t *testing.T) {
	policy, _ := NewPolicy("policy-1", "Test", "Description", "tenant", ActionDeny, 100)
	_ = policy.AddCondition("tenant_id", "eq", "default")

	policy.Disable()

	attributes := map[string]interface{}{
		"tenant_id": "default",
	}

	matches, _ := policy.Evaluate(attributes)
	if matches {
		t.Error("Disabled policy should not match")
	}
}
