package usecases

import (
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func TestCreateTenantUseCase_Execute(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	useCase := NewCreateTenantUseCase(tenantRepo, auditRepo)

	t.Run("Create new tenant", func(t *testing.T) {
		req := dto.CreateTenantRequest{
			ID:          "tenant-1",
			Name:        "Test Tenant",
			Description: "Test description",
		}

		resp, err := useCase.Execute(req)
		if err != nil {
			t.Fatalf("Execute() failed: %v", err)
		}

		if resp.ID != req.ID {
			t.Errorf("Response.ID = %v, want %v", resp.ID, req.ID)
		}
		if resp.Name != req.Name {
			t.Errorf("Response.Name = %v, want %v", resp.Name, req.Name)
		}
		if resp.Status != "active" {
			t.Errorf("Response.Status = %v, want active", resp.Status)
		}
	})

	t.Run("Create duplicate tenant", func(t *testing.T) {
		req := dto.CreateTenantRequest{
			ID:          "tenant-1",
			Name:        "Duplicate Tenant",
			Description: "Duplicate",
		}

		_, err := useCase.Execute(req)
		if err != domain.ErrTenantAlreadyExists {
			t.Errorf("Expected ErrTenantAlreadyExists, got %v", err)
		}
	})

	t.Run("Create tenant with invalid ID", func(t *testing.T) {
		req := dto.CreateTenantRequest{
			ID:          "",
			Name:        "Invalid Tenant",
			Description: "Invalid",
		}

		_, err := useCase.Execute(req)
		if err == nil {
			t.Error("Expected error for empty tenant ID")
		}
	})
}
