package entities

import (
	"testing"
)

func TestNewSLO(t *testing.T) {
	slo, err := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days)
	if err != nil {
		t.Fatalf("NewSLO() failed: %v", err)
	}

	if slo.ID != "slo-1" {
		t.Errorf("SLO.ID = %v, want slo-1", slo.ID)
	}
	if slo.Name != "API Availability" {
		t.Errorf("SLO.Name = %v, want API Availability", slo.Name)
	}
	if slo.TargetPercent != 99.9 {
		t.Errorf("SLO.TargetPercent = %v, want 99.9", slo.TargetPercent)
	}
	if slo.Window != SLOWindow30Days {
		t.Errorf("SLO.Window = %v, want 30d", slo.Window)
	}
}

func TestNewSLO_Validation(t *testing.T) {
	tests := []struct {
		name    string
		id      string
		sname   string
		metric  string
		target  float64
		window  SLOWindow
		wantErr bool
	}{
		{"empty ID", "", "name", "metric", 99.9, SLOWindow30Days, true},
		{"empty name", "id", "", "metric", 99.9, SLOWindow30Days, true},
		{"empty metric", "id", "name", "", 99.9, SLOWindow30Days, true},
		{"zero target", "id", "name", "metric", 0, SLOWindow30Days, true},
		{"negative target", "id", "name", "metric", -1, SLOWindow30Days, true},
		{"target over 100", "id", "name", "metric", 101, SLOWindow30Days, true},
		{"invalid window", "id", "name", "metric", 99.9, "1d", true},
		{"valid 7d", "id", "name", "metric", 99.9, SLOWindow7Days, false},
		{"valid 30d", "id", "name", "metric", 99.0, SLOWindow30Days, false},
		{"target exactly 100", "id", "name", "metric", 100, SLOWindow30Days, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := NewSLO(tt.id, tt.sname, tt.metric, tt.target, tt.window)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewSLO() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestSLO_CalculateCompliance_Compliant(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	compliance := slo.CalculateCompliance(99.95)

	if !compliance.IsCompliant {
		t.Error("SLO should be compliant when current >= target")
	}
	if compliance.CurrentPercent != 99.95 {
		t.Errorf("CurrentPercent = %v, want 99.95", compliance.CurrentPercent)
	}
	// Error budget total: 100 - 99.9 = 0.1
	// Error budget used: 100 - 99.95 = 0.05
	// Remaining: 0.1 - 0.05 = 0.05
	if compliance.RemainingBudget < 0.049 || compliance.RemainingBudget > 0.051 {
		t.Errorf("RemainingBudget = %v, want ~0.05", compliance.RemainingBudget)
	}
}

func TestSLO_CalculateCompliance_NonCompliant(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	compliance := slo.CalculateCompliance(99.5)

	if compliance.IsCompliant {
		t.Error("SLO should not be compliant when current < target")
	}
	// Error budget total: 100 - 99.9 = 0.1
	// Error budget used: 100 - 99.5 = 0.5
	// Remaining: 0.1 - 0.5 = -0.4 => clamped to 0
	if compliance.RemainingBudget != 0 {
		t.Errorf("RemainingBudget = %v, want 0 (budget exhausted)", compliance.RemainingBudget)
	}
}

func TestSLO_CalculateCompliance_ExactlyOnTarget(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	compliance := slo.CalculateCompliance(99.9)

	if !compliance.IsCompliant {
		t.Error("SLO should be compliant when current == target")
	}
}

func TestSLO_Update(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	target := 99.5
	err := slo.Update("Updated SLO", "", &target, SLOWindow7Days)
	if err != nil {
		t.Fatalf("Update() failed: %v", err)
	}

	if slo.Name != "Updated SLO" {
		t.Errorf("SLO.Name = %v, want Updated SLO", slo.Name)
	}
	if slo.Metric != "availability" {
		t.Errorf("SLO.Metric should not change, got %v", slo.Metric)
	}
	if slo.TargetPercent != 99.5 {
		t.Errorf("SLO.TargetPercent = %v, want 99.5", slo.TargetPercent)
	}
	if slo.Window != SLOWindow7Days {
		t.Errorf("SLO.Window = %v, want 7d", slo.Window)
	}
}

func TestSLO_Update_InvalidTarget(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	target := 0.0
	err := slo.Update("", "", &target, "")
	if err == nil {
		t.Error("Update() should fail with invalid target")
	}
}

func TestSLO_Update_InvalidWindow(t *testing.T) {
	slo, _ := NewSLO("slo-1", "API Availability", "availability", 99.9, SLOWindow30Days) //nolint:errcheck

	err := slo.Update("", "", nil, "1d")
	if err == nil {
		t.Error("Update() should fail with invalid window")
	}
}

func TestValidateSLOWindow(t *testing.T) {
	tests := []struct {
		window  SLOWindow
		wantErr bool
	}{
		{SLOWindow7Days, false},
		{SLOWindow30Days, false},
		{"1d", true},
		{"", true},
		{"90d", true},
	}

	for _, tt := range tests {
		t.Run(string(tt.window), func(t *testing.T) {
			err := ValidateSLOWindow(tt.window)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateSLOWindow(%v) error = %v, wantErr %v", tt.window, err, tt.wantErr)
			}
		})
	}
}
