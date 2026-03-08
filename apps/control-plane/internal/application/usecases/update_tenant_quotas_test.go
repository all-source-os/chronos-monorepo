package usecases

import (
	"context"
	"errors"
	"testing"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func int64Ptr(v int64) *int64 { return &v }

func TestUpdateTenantQuotasUseCase_Execute(t *testing.T) {
	tenantRepo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	uc := NewUpdateTenantQuotasUseCase(tenantRepo, auditRepo)

	// Seed a tenant
	tenant, _ := entities.NewTenant("t-quota-1", "Quota Test", "test tenant") //nolint:errcheck
	_ = tenantRepo.Save(tenant)                                               //nolint:errcheck

	tests := []struct {
		name      string
		tenantID  string
		role      entities.Role
		req       dto.UpdateTenantQuotasRequest
		wantErr   bool
		errTarget error
		checkFn   func(t *testing.T, resp *dto.TenantResponse)
	}{
		{
			name:     "successful quota update — all fields",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit:     int64Ptr(10000),
				QueryLimit:     int64Ptr(5000),
				StorageLimitMB: int64Ptr(1024),
			},
			checkFn: func(t *testing.T, resp *dto.TenantResponse) {
				if resp.Quotas == nil {
					t.Fatal("expected quotas in response")
				}
				if resp.Quotas.EventLimit != 10000 {
					t.Errorf("expected event_limit 10000, got %d", resp.Quotas.EventLimit)
				}
				if resp.Quotas.QueryLimit != 5000 {
					t.Errorf("expected query_limit 5000, got %d", resp.Quotas.QueryLimit)
				}
				if resp.Quotas.StorageLimitMB != 1024 {
					t.Errorf("expected storage_limit_mb 1024, got %d", resp.Quotas.StorageLimitMB)
				}
			},
		},
		{
			name:     "partial update — only event_limit",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit: int64Ptr(20000),
			},
			checkFn: func(t *testing.T, resp *dto.TenantResponse) {
				if resp.Quotas.EventLimit != 20000 {
					t.Errorf("expected event_limit 20000, got %d", resp.Quotas.EventLimit)
				}
				// Previous values should be preserved
				if resp.Quotas.QueryLimit != 5000 {
					t.Errorf("expected query_limit 5000 (preserved), got %d", resp.Quotas.QueryLimit)
				}
				if resp.Quotas.StorageLimitMB != 1024 {
					t.Errorf("expected storage_limit_mb 1024 (preserved), got %d", resp.Quotas.StorageLimitMB)
				}
			},
		},
		{
			name:     "negative event_limit rejected",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit: int64Ptr(-1),
			},
			wantErr:   true,
			errTarget: domain.ErrInvalidInput,
		},
		{
			name:     "negative query_limit rejected",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				QueryLimit: int64Ptr(-100),
			},
			wantErr:   true,
			errTarget: domain.ErrInvalidInput,
		},
		{
			name:     "negative storage_limit_mb rejected",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				StorageLimitMB: int64Ptr(-50),
			},
			wantErr:   true,
			errTarget: domain.ErrInvalidInput,
		},
		{
			name:     "zero values are valid (unlimited)",
			tenantID: "t-quota-1",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit:     int64Ptr(0),
				QueryLimit:     int64Ptr(0),
				StorageLimitMB: int64Ptr(0),
			},
			checkFn: func(t *testing.T, resp *dto.TenantResponse) {
				if resp.Quotas.EventLimit != 0 {
					t.Errorf("expected event_limit 0, got %d", resp.Quotas.EventLimit)
				}
			},
		},
		{
			name:     "non-admin role rejected",
			tenantID: "t-quota-1",
			role:     entities.RoleDeveloper,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit: int64Ptr(100),
			},
			wantErr:   true,
			errTarget: domain.ErrForbidden,
		},
		{
			name:     "tenant not found",
			tenantID: "nonexistent",
			role:     entities.RoleAdmin,
			req: dto.UpdateTenantQuotasRequest{
				EventLimit: int64Ptr(100),
			},
			wantErr:   true,
			errTarget: domain.ErrTenantNotFound,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			resp, err := uc.Execute(context.Background(), tt.tenantID, tt.req, tt.role)
			if tt.wantErr {
				if err == nil {
					t.Fatal("expected error, got nil")
				}
				if tt.errTarget != nil && !containsError(err, tt.errTarget) {
					t.Errorf("expected error containing %v, got %v", tt.errTarget, err)
				}
				return
			}
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if tt.checkFn != nil {
				tt.checkFn(t, resp)
			}
		})
	}
}

// containsError checks if err wraps or equals target.
func containsError(err, target error) bool {
	if errors.Is(err, target) {
		return true
	}
	// Also check string containment for wrapped errors using fmt.Errorf %w
	return err.Error() == target.Error() || len(err.Error()) > len(target.Error()) && err.Error()[:len(target.Error())] == target.Error()
}
