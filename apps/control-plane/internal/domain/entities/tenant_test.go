package entities

import (
	"testing"
)

func TestNewTenant(t *testing.T) {
	tests := []struct {
		name        string
		id          string
		tenantName  string
		description string
		wantErr     bool
	}{
		{
			name:        "Valid tenant",
			id:          "tenant-1",
			tenantName:  "Test Tenant",
			description: "Test description",
			wantErr:     false,
		},
		{
			name:        "Empty ID",
			id:          "",
			tenantName:  "Test Tenant",
			description: "Test description",
			wantErr:     true,
		},
		{
			name:        "Empty name",
			id:          "tenant-1",
			tenantName:  "",
			description: "Test description",
			wantErr:     true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tenant, err := NewTenant(tt.id, tt.tenantName, tt.description)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewTenant() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr {
				if tenant.ID != tt.id {
					t.Errorf("Tenant.ID = %v, want %v", tenant.ID, tt.id)
				}
				if tenant.Name != tt.tenantName {
					t.Errorf("Tenant.Name = %v, want %v", tenant.Name, tt.tenantName)
				}
				if tenant.Status != TenantStatusActive {
					t.Errorf("Tenant.Status = %v, want %v", tenant.Status, TenantStatusActive)
				}
			}
		})
	}
}

func TestTenant_IsActive(t *testing.T) {
	tenant, _ := NewTenant("tenant-1", "Test", "Description")

	if !tenant.IsActive() {
		t.Error("New tenant should be active")
	}

	tenant.Suspend()
	if tenant.IsActive() {
		t.Error("Suspended tenant should not be active")
	}

	tenant.Activate()
	if !tenant.IsActive() {
		t.Error("Activated tenant should be active")
	}
}

func TestTenant_MarkDeleted(t *testing.T) {
	t.Run("Delete default tenant", func(t *testing.T) {
		tenant, _ := NewTenant("default", "Default", "Default tenant")
		err := tenant.MarkDeleted()
		if err == nil {
			t.Error("Should not be able to delete default tenant")
		}
	})

	t.Run("Delete non-default tenant", func(t *testing.T) {
		tenant, _ := NewTenant("tenant-1", "Test", "Test tenant")
		err := tenant.MarkDeleted()
		if err != nil {
			t.Errorf("Should be able to delete non-default tenant: %v", err)
		}
		if tenant.Status != TenantStatusDeleted {
			t.Errorf("Status should be deleted, got %v", tenant.Status)
		}
	})
}
