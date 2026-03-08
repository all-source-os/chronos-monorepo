package usecases

import (
	"errors"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// stubMetricsProvider implements MetricsProvider for testing
type stubMetricsProvider struct {
	metrics map[string]float64
}

func (s *stubMetricsProvider) GetCurrentMetricPercent(metric string, _ time.Duration) (float64, error) {
	if val, ok := s.metrics[metric]; ok {
		return val, nil
	}
	return 100.0, nil
}

func TestCreateSLOUseCase(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewCreateSLOUseCase(sloRepo, auditRepo)

	req := dto.CreateSLORequest{
		Name:          "API Availability",
		Metric:        "availability",
		TargetPercent: 99.9,
		Window:        "30d",
	}

	resp, err := uc.Execute(req)
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}

	if resp.Name != "API Availability" {
		t.Errorf("Name = %v, want API Availability", resp.Name)
	}
	if resp.TargetPercent != 99.9 {
		t.Errorf("TargetPercent = %v, want 99.9", resp.TargetPercent)
	}
	if resp.Window != "30d" {
		t.Errorf("Window = %v, want 30d", resp.Window)
	}
	if resp.ID == "" {
		t.Error("ID should not be empty")
	}
}

func TestCreateSLOUseCase_InvalidWindow(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewCreateSLOUseCase(sloRepo, auditRepo)

	req := dto.CreateSLORequest{
		Name:          "Bad SLO",
		Metric:        "availability",
		TargetPercent: 99.9,
		Window:        "1d",
	}

	_, err := uc.Execute(req)
	if err == nil {
		t.Error("Execute() should fail with invalid window")
	}
}

func TestCreateSLOUseCase_InvalidTarget(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewCreateSLOUseCase(sloRepo, auditRepo)

	req := dto.CreateSLORequest{
		Name:          "Bad SLO",
		Metric:        "availability",
		TargetPercent: 101.0,
		Window:        "30d",
	}

	_, err := uc.Execute(req)
	if err == nil {
		t.Error("Execute() should fail with target > 100")
	}
}

func TestListSLOsUseCase_WithMetricsProvider(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateSLOUseCase(sloRepo, auditRepo)

	provider := &stubMetricsProvider{
		metrics: map[string]float64{
			"availability": 99.95,
			"latency_p99":  98.0,
		},
	}
	listUC := NewListSLOsUseCase(sloRepo, provider)

	// Create two SLOs
	_, _ = createUC.Execute(dto.CreateSLORequest{ //nolint:errcheck
		Name:          "API Availability",
		Metric:        "availability",
		TargetPercent: 99.9,
		Window:        "30d",
	})
	_, _ = createUC.Execute(dto.CreateSLORequest{ //nolint:errcheck
		Name:          "Latency P99",
		Metric:        "latency_p99",
		TargetPercent: 99.0,
		Window:        "7d",
	})

	results, err := listUC.Execute()
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if len(results) != 2 {
		t.Fatalf("Expected 2 SLOs, got %d", len(results))
	}

	// Find results by name since order is not guaranteed in map iteration
	for _, r := range results {
		switch r.Name {
		case "API Availability":
			if !r.IsCompliant {
				t.Error("API Availability should be compliant (99.95 >= 99.9)")
			}
			if r.CurrentPercent != 99.95 {
				t.Errorf("CurrentPercent = %v, want 99.95", r.CurrentPercent)
			}
		case "Latency P99":
			if r.IsCompliant {
				t.Error("Latency P99 should not be compliant (98.0 < 99.0)")
			}
			if r.CurrentPercent != 98.0 {
				t.Errorf("CurrentPercent = %v, want 98.0", r.CurrentPercent)
			}
			if r.RemainingBudget != 0 {
				t.Errorf("RemainingBudget = %v, want 0 (budget exhausted)", r.RemainingBudget)
			}
		}
	}
}

func TestListSLOsUseCase_WithoutMetricsProvider(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateSLOUseCase(sloRepo, auditRepo)
	listUC := NewListSLOsUseCase(sloRepo, nil)

	_, _ = createUC.Execute(dto.CreateSLORequest{ //nolint:errcheck
		Name:          "API Availability",
		Metric:        "availability",
		TargetPercent: 99.9,
		Window:        "30d",
	})

	results, err := listUC.Execute()
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("Expected 1 SLO, got %d", len(results))
	}

	// Without metrics provider, defaults to 100% (always compliant)
	if !results[0].IsCompliant {
		t.Error("SLO should be compliant when no metrics provider")
	}
	if results[0].CurrentPercent != 100.0 {
		t.Errorf("CurrentPercent = %v, want 100.0", results[0].CurrentPercent)
	}
}

func TestDeleteSLOUseCase(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	createUC := NewCreateSLOUseCase(sloRepo, auditRepo)
	deleteUC := NewDeleteSLOUseCase(sloRepo, auditRepo)
	listUC := NewListSLOsUseCase(sloRepo, nil)

	resp, _ := createUC.Execute(dto.CreateSLORequest{ //nolint:errcheck
		Name:          "API Availability",
		Metric:        "availability",
		TargetPercent: 99.9,
		Window:        "30d",
	})

	err := deleteUC.Execute(resp.ID)
	if err != nil {
		t.Fatalf("Execute() failed: %v", err)
	}

	results, _ := listUC.Execute() //nolint:errcheck
	if len(results) != 0 {
		t.Errorf("Expected 0 SLOs after delete, got %d", len(results))
	}
}

func TestDeleteSLOUseCase_NotFound(t *testing.T) {
	sloRepo := persistence.NewMemorySLORepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	deleteUC := NewDeleteSLOUseCase(sloRepo, auditRepo)

	err := deleteUC.Execute("nonexistent")
	if err == nil {
		t.Error("Execute() should fail for nonexistent SLO")
	}
	if !errors.Is(err, domain.ErrSLONotFound) {
		t.Errorf("Expected ErrSLONotFound, got %v", err)
	}
}
