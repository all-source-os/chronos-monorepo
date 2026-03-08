package usecases

import (
	"errors"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestCreateAlertRuleUseCase(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewCreateAlertRuleUseCase(alertRepo, auditRepo)

	req := dto.CreateAlertRuleRequest{
		Name:                "High CPU",
		Metric:              "cpu_usage",
		Operator:            ">",
		Threshold:           90.0,
		Duration:            "5m",
		NotificationChannel: "slack",
	}

	resp, err := uc.Execute(req)
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}

	if resp.Name != "High CPU" {
		t.Errorf("Name = %v, want High CPU", resp.Name)
	}
	if resp.Operator != ">" {
		t.Errorf("Operator = %v, want >", resp.Operator)
	}
	if resp.Threshold != 90.0 {
		t.Errorf("Threshold = %v, want 90.0", resp.Threshold)
	}
	if !resp.Enabled {
		t.Error("New alert rule should be enabled")
	}
	if resp.ID == "" {
		t.Error("ID should not be empty")
	}
}

func TestCreateAlertRuleUseCase_InvalidOperator(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewCreateAlertRuleUseCase(alertRepo, auditRepo)

	req := dto.CreateAlertRuleRequest{
		Name:                "Bad Rule",
		Metric:              "cpu_usage",
		Operator:            "!=",
		Threshold:           90.0,
		Duration:            "5m",
		NotificationChannel: "slack",
	}

	_, err := uc.Execute(req)
	if err == nil {
		t.Error("Execute() should fail with invalid operator")
	}
}

func TestListAlertRulesUseCase(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateAlertRuleUseCase(alertRepo, auditRepo)
	listUC := NewListAlertRulesUseCase(alertRepo)

	// Create two rules
	req1 := dto.CreateAlertRuleRequest{
		Name:                "High CPU",
		Metric:              "cpu_usage",
		Operator:            ">",
		Threshold:           90.0,
		Duration:            "5m",
		NotificationChannel: "slack",
	}
	req2 := dto.CreateAlertRuleRequest{
		Name:                "Low Disk",
		Metric:              "disk_free",
		Operator:            "<",
		Threshold:           10.0,
		Duration:            "1h",
		NotificationChannel: "email",
	}

	_, _ = createUC.Execute(req1) //nolint:errcheck
	_, _ = createUC.Execute(req2) //nolint:errcheck

	rules, err := listUC.Execute()
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if len(rules) != 2 {
		t.Errorf("Expected 2 rules, got %d", len(rules))
	}
}

func TestUpdateAlertRuleUseCase(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateAlertRuleUseCase(alertRepo, auditRepo)
	updateUC := NewUpdateAlertRuleUseCase(alertRepo, auditRepo)

	resp, _ := createUC.Execute(dto.CreateAlertRuleRequest{ //nolint:errcheck
		Name:                "High CPU",
		Metric:              "cpu_usage",
		Operator:            ">",
		Threshold:           90.0,
		Duration:            "5m",
		NotificationChannel: "slack",
	})

	threshold := 95.0
	updated, err := updateUC.Execute(resp.ID, dto.UpdateAlertRuleRequest{
		Name:      "Very High CPU",
		Threshold: &threshold,
	})
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if updated.Name != "Very High CPU" {
		t.Errorf("Name = %v, want Very High CPU", updated.Name)
	}
	if updated.Threshold != 95.0 {
		t.Errorf("Threshold = %v, want 95.0", updated.Threshold)
	}
}

func TestUpdateAlertRuleUseCase_NotFound(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	updateUC := NewUpdateAlertRuleUseCase(alertRepo, auditRepo)

	_, err := updateUC.Execute("nonexistent", dto.UpdateAlertRuleRequest{Name: "test"})
	if err == nil {
		t.Error("Execute() should fail for nonexistent rule")
	}
	if !errors.Is(err, domain.ErrAlertRuleNotFound) {
		t.Errorf("Expected ErrAlertRuleNotFound, got %v", err)
	}
}

func TestDeleteAlertRuleUseCase(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateAlertRuleUseCase(alertRepo, auditRepo)
	deleteUC := NewDeleteAlertRuleUseCase(alertRepo, auditRepo)
	listUC := NewListAlertRulesUseCase(alertRepo)

	resp, _ := createUC.Execute(dto.CreateAlertRuleRequest{ //nolint:errcheck
		Name:                "High CPU",
		Metric:              "cpu_usage",
		Operator:            ">",
		Threshold:           90.0,
		Duration:            "5m",
		NotificationChannel: "slack",
	})

	err := deleteUC.Execute(resp.ID)
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}

	rules, _ := listUC.Execute() //nolint:errcheck
	if len(rules) != 0 {
		t.Errorf("Expected 0 rules after delete, got %d", len(rules))
	}
}

func TestDeleteAlertRuleUseCase_NotFound(t *testing.T) {
	alertRepo := persistence.NewMemoryAlertRuleRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	deleteUC := NewDeleteAlertRuleUseCase(alertRepo, auditRepo)

	err := deleteUC.Execute("nonexistent")
	if err == nil {
		t.Error("Execute() should fail for nonexistent rule")
	}
	if !errors.Is(err, domain.ErrAlertRuleNotFound) {
		t.Errorf("Expected ErrAlertRuleNotFound, got %v", err)
	}
}
