package entities

import (
	"testing"
)

func TestNewIPRule(t *testing.T) {
	rule, err := NewIPRule("rule-1", "192.168.1.0/24", IPRuleTypeAllow, "Office network", "admin")
	if err != nil {
		t.Fatalf("NewIPRule() failed: %v", err)
	}

	if rule.ID != "rule-1" {
		t.Errorf("IPRule.ID = %v, want rule-1", rule.ID)
	}
	if rule.CIDR != "192.168.1.0/24" {
		t.Errorf("IPRule.CIDR = %v, want 192.168.1.0/24", rule.CIDR)
	}
	if rule.Type != IPRuleTypeAllow {
		t.Errorf("IPRule.Type = %v, want allow", rule.Type)
	}
	if rule.Description != "Office network" {
		t.Errorf("IPRule.Description = %v, want Office network", rule.Description)
	}
	if rule.CreatedBy != "admin" {
		t.Errorf("IPRule.CreatedBy = %v, want admin", rule.CreatedBy)
	}
	if rule.CreatedAt.IsZero() {
		t.Error("IPRule.CreatedAt should not be zero")
	}
}

func TestNewIPRule_BlockType(t *testing.T) {
	rule, err := NewIPRule("rule-2", "10.0.0.0/8", IPRuleTypeBlock, "Block internal", "admin")
	if err != nil {
		t.Fatalf("NewIPRule() failed: %v", err)
	}
	if rule.Type != IPRuleTypeBlock {
		t.Errorf("IPRule.Type = %v, want block", rule.Type)
	}
}

func TestNewIPRule_Validation(t *testing.T) {
	tests := []struct {
		name        string
		id          string
		cidr        string
		ruleType    IPRuleType
		description string
		createdBy   string
		wantErr     bool
	}{
		{"empty ID", "", "192.168.1.0/24", IPRuleTypeAllow, "desc", "admin", true},
		{"empty CIDR", "id", "", IPRuleTypeAllow, "desc", "admin", true},
		{"invalid CIDR", "id", "not-a-cidr", IPRuleTypeAllow, "desc", "admin", true},
		{"invalid CIDR partial", "id", "999.999.999.999/24", IPRuleTypeAllow, "desc", "admin", true},
		{"invalid type", "id", "10.0.0.0/8", "invalid", "desc", "admin", true},
		{"valid IPv4 CIDR", "id", "10.0.0.0/8", IPRuleTypeAllow, "desc", "admin", false},
		{"valid IPv4 /32", "id", "192.168.1.1/32", IPRuleTypeBlock, "desc", "admin", false},
		{"valid IPv6 CIDR", "id", "2001:db8::/32", IPRuleTypeAllow, "desc", "admin", false},
		{"valid plain IP", "id", "192.168.1.1", IPRuleTypeAllow, "desc", "admin", false},
		{"valid block type", "id", "10.0.0.0/16", IPRuleTypeBlock, "desc", "admin", false},
		{"empty description ok", "id", "10.0.0.0/8", IPRuleTypeAllow, "", "admin", false},
		{"empty createdBy ok", "id", "10.0.0.0/8", IPRuleTypeAllow, "desc", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := NewIPRule(tt.id, tt.cidr, tt.ruleType, tt.description, tt.createdBy)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewIPRule() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidateCIDR(t *testing.T) {
	tests := []struct {
		cidr    string
		wantErr bool
	}{
		{"192.168.1.0/24", false},
		{"10.0.0.0/8", false},
		{"172.16.0.0/12", false},
		{"0.0.0.0/0", false},
		{"192.168.1.1/32", false},
		{"2001:db8::/32", false},
		{"::1/128", false},
		{"192.168.1.1", false},   // plain IP accepted
		{"::1", false},           // plain IPv6 accepted
		{"", true},               // empty
		{"not-a-cidr", true},     // garbage
		{"192.168.1.0/33", true}, // invalid prefix length
		{"192.168.1.0/", true},   // incomplete
	}

	for _, tt := range tests {
		t.Run(tt.cidr, func(t *testing.T) {
			err := ValidateCIDR(tt.cidr)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateCIDR(%q) error = %v, wantErr %v", tt.cidr, err, tt.wantErr)
			}
		})
	}
}

func TestValidateIPRuleType(t *testing.T) {
	tests := []struct {
		ruleType IPRuleType
		wantErr  bool
	}{
		{IPRuleTypeAllow, false},
		{IPRuleTypeBlock, false},
		{"invalid", true},
		{"", true},
		{"ALLOW", true},
	}

	for _, tt := range tests {
		t.Run(string(tt.ruleType), func(t *testing.T) {
			err := ValidateIPRuleType(tt.ruleType)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateIPRuleType(%v) error = %v, wantErr %v", tt.ruleType, err, tt.wantErr)
			}
		})
	}
}
