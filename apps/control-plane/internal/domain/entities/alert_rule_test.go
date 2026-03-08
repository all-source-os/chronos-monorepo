package entities

import (
	"testing"
)

func TestNewAlertRule(t *testing.T) {
	rule, err := NewAlertRule("alert-1", "High CPU", "cpu_usage", OperatorGreaterThan, 90.0, "5m", "slack")
	if err != nil {
		t.Fatalf("NewAlertRule() failed: %v", err)
	}

	if rule.ID != "alert-1" {
		t.Errorf("AlertRule.ID = %v, want alert-1", rule.ID)
	}
	if rule.Name != "High CPU" {
		t.Errorf("AlertRule.Name = %v, want High CPU", rule.Name)
	}
	if rule.Metric != "cpu_usage" {
		t.Errorf("AlertRule.Metric = %v, want cpu_usage", rule.Metric)
	}
	if rule.Operator != OperatorGreaterThan {
		t.Errorf("AlertRule.Operator = %v, want >", rule.Operator)
	}
	if rule.Threshold != 90.0 {
		t.Errorf("AlertRule.Threshold = %v, want 90.0", rule.Threshold)
	}
	if rule.Duration != "5m" {
		t.Errorf("AlertRule.Duration = %v, want 5m", rule.Duration)
	}
	if rule.NotificationChannel != "slack" {
		t.Errorf("AlertRule.NotificationChannel = %v, want slack", rule.NotificationChannel)
	}
	if !rule.Enabled {
		t.Error("New alert rule should be enabled")
	}
}

func TestNewAlertRule_Validation(t *testing.T) {
	tests := []struct {
		name    string
		id      string
		rname   string
		metric  string
		op      AlertOperator
		dur     string
		channel string
		wantErr bool
	}{
		{"empty ID", "", "name", "metric", OperatorGreaterThan, "5m", "slack", true},
		{"empty name", "id", "", "metric", OperatorGreaterThan, "5m", "slack", true},
		{"empty metric", "id", "name", "", OperatorGreaterThan, "5m", "slack", true},
		{"invalid operator", "id", "name", "metric", "!=", "5m", "slack", true},
		{"empty duration", "id", "name", "metric", OperatorGreaterThan, "", "slack", true},
		{"empty channel", "id", "name", "metric", OperatorGreaterThan, "5m", "", true},
		{"valid", "id", "name", "metric", OperatorLessThan, "5m", "email", false},
		{"valid equal", "id", "name", "metric", OperatorEqual, "1h", "pager", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := NewAlertRule(tt.id, tt.rname, tt.metric, tt.op, 50.0, tt.dur, tt.channel)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewAlertRule() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestAlertRule_Update(t *testing.T) {
	rule, _ := NewAlertRule("alert-1", "High CPU", "cpu_usage", OperatorGreaterThan, 90.0, "5m", "slack") //nolint:errcheck

	threshold := 95.0
	enabled := false
	err := rule.Update("Very High CPU", "", "", &threshold, "10m", "", &enabled)
	if err != nil {
		t.Fatalf("Update() failed: %v", err)
	}

	if rule.Name != "Very High CPU" {
		t.Errorf("AlertRule.Name = %v, want Very High CPU", rule.Name)
	}
	if rule.Metric != "cpu_usage" {
		t.Errorf("AlertRule.Metric should not change, got %v", rule.Metric)
	}
	if rule.Threshold != 95.0 {
		t.Errorf("AlertRule.Threshold = %v, want 95.0", rule.Threshold)
	}
	if rule.Duration != "10m" {
		t.Errorf("AlertRule.Duration = %v, want 10m", rule.Duration)
	}
	if rule.Enabled {
		t.Error("AlertRule should be disabled after update")
	}
}

func TestAlertRule_Update_InvalidOperator(t *testing.T) {
	rule, _ := NewAlertRule("alert-1", "High CPU", "cpu_usage", OperatorGreaterThan, 90.0, "5m", "slack") //nolint:errcheck

	err := rule.Update("", "", "invalid_op", nil, "", "", nil)
	if err == nil {
		t.Error("Update() should fail with invalid operator")
	}
}

func TestAlertRule_EnableDisable(t *testing.T) {
	rule, _ := NewAlertRule("alert-1", "High CPU", "cpu_usage", OperatorGreaterThan, 90.0, "5m", "slack") //nolint:errcheck

	rule.Disable()
	if rule.Enabled {
		t.Error("AlertRule should be disabled")
	}

	rule.Enable()
	if !rule.Enabled {
		t.Error("AlertRule should be enabled")
	}
}

func TestValidateAlertOperator(t *testing.T) {
	tests := []struct {
		op      AlertOperator
		wantErr bool
	}{
		{OperatorGreaterThan, false},
		{OperatorLessThan, false},
		{OperatorEqual, false},
		{"!=", true},
		{"", true},
		{">=", true},
	}

	for _, tt := range tests {
		t.Run(string(tt.op), func(t *testing.T) {
			err := ValidateAlertOperator(tt.op)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateAlertOperator(%v) error = %v, wantErr %v", tt.op, err, tt.wantErr)
			}
		})
	}
}
