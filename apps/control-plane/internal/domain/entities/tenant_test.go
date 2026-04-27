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
	tenant, _ := NewTenant("tenant-1", "Test", "Description") //nolint:errcheck // test code

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

func TestTenantQuotas_Validate(t *testing.T) {
	tests := []struct {
		name    string
		quotas  TenantQuotas
		wantErr bool
		errMsg  string
	}{
		{
			name:    "all zeros (unlimited) — valid",
			quotas:  TenantQuotas{EventLimit: 0, QueryLimit: 0, StorageLimitMB: 0},
			wantErr: false,
		},
		{
			name:    "positive values — valid",
			quotas:  TenantQuotas{EventLimit: 10000, QueryLimit: 5000, StorageLimitMB: 1024},
			wantErr: false,
		},
		{
			name:    "negative event_limit — invalid",
			quotas:  TenantQuotas{EventLimit: -1, QueryLimit: 0, StorageLimitMB: 0},
			wantErr: true,
			errMsg:  "event_limit must not be negative",
		},
		{
			name:    "negative query_limit — invalid",
			quotas:  TenantQuotas{EventLimit: 0, QueryLimit: -1, StorageLimitMB: 0},
			wantErr: true,
			errMsg:  "query_limit must not be negative",
		},
		{
			name:    "negative storage_limit_mb — invalid",
			quotas:  TenantQuotas{EventLimit: 0, QueryLimit: 0, StorageLimitMB: -1},
			wantErr: true,
			errMsg:  "storage_limit_mb must not be negative",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.quotas.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
			if tt.wantErr && err != nil && err.Error() != tt.errMsg {
				t.Errorf("Validate() error message = %q, want %q", err.Error(), tt.errMsg)
			}
		})
	}
}

func TestTenant_UpdateQuotas(t *testing.T) {
	tenant, _ := NewTenant("t-1", "Test", "desc") //nolint:errcheck // test code

	// Valid update
	err := tenant.UpdateQuotas(TenantQuotas{EventLimit: 1000, QueryLimit: 500, StorageLimitMB: 256})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tenant.Quotas.EventLimit != 1000 {
		t.Errorf("expected EventLimit 1000, got %d", tenant.Quotas.EventLimit)
	}
	if tenant.Quotas.QueryLimit != 500 {
		t.Errorf("expected QueryLimit 500, got %d", tenant.Quotas.QueryLimit)
	}
	if tenant.Quotas.StorageLimitMB != 256 {
		t.Errorf("expected StorageLimitMB 256, got %d", tenant.Quotas.StorageLimitMB)
	}

	// Invalid update — should not change existing quotas
	err = tenant.UpdateQuotas(TenantQuotas{EventLimit: -1})
	if err == nil {
		t.Fatal("expected error for negative quota")
	}
	// Quotas should remain unchanged
	if tenant.Quotas.EventLimit != 1000 {
		t.Errorf("quotas should not change on invalid update, got EventLimit %d", tenant.Quotas.EventLimit)
	}
}

func TestTenant_MarkDeleted(t *testing.T) {
	t.Run("Delete default tenant", func(t *testing.T) {
		tenant, _ := NewTenant("default", "Default", "Default tenant") //nolint:errcheck // test code
		err := tenant.MarkDeleted()
		if err == nil {
			t.Error("Should not be able to delete default tenant")
		}
	})

	t.Run("Delete non-default tenant", func(t *testing.T) {
		tenant, _ := NewTenant("tenant-1", "Test", "Test tenant") //nolint:errcheck // test code
		err := tenant.MarkDeleted()
		if err != nil {
			t.Errorf("Should be able to delete non-default tenant: %v", err)
		}
		if tenant.Status != TenantStatusDeleted {
			t.Errorf("Status should be deleted, got %v", tenant.Status)
		}
	})
}

func TestTenantSlug(t *testing.T) {
	tests := []struct {
		name string
		in   string
		want string
	}{
		{"email", "alice@example.com", "alice-at-example-com"},
		{"oauth userid with colons", "oauth:github:12345", "oauth-github-12345"},
		{"oauth email userid", "oauth:email:user@example.com", "oauth-email-user-at-example-com"},
		{"mixed case", "Alice@Example.COM", "alice-at-example-com"},
		{"spaces", "Hello World", "hello-world"},
		{"collapses double hyphens", "foo..bar", "foo-bar"},
		{"trims leading/trailing hyphens", ":foo:", "foo"},
		{"alphanumeric preserved", "abc123_xyz", "abc123_xyz"},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			got := TenantSlug(tc.in)
			if got != tc.want {
				t.Errorf("TenantSlug(%q) = %q, want %q", tc.in, got, tc.want)
			}
			// Output must only contain alphanumerics, hyphens, underscores (Core's validation).
			for _, r := range got {
				ok := (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') || r == '-' || r == '_'
				if !ok {
					t.Errorf("TenantSlug(%q) = %q contains invalid char %q", tc.in, got, r)
				}
			}
		})
	}
}

// Step 1 of REGIONAL_INDEPENDENCE.md: home_region attribute on the
// tenant entity. The tests below cover the contract every later
// step depends on — default region, allowlist enforcement, the
// EffectiveHomeRegion fallback for legacy rows, and the validated
// SetHomeRegion mutation.

func TestNewTenant_DefaultsToIad(t *testing.T) {
	tenant, err := NewTenant("acme", "Acme Co", "")
	if err != nil {
		t.Fatalf("NewTenant returned error: %v", err)
	}
	if tenant.HomeRegion != DefaultHomeRegion {
		t.Errorf("HomeRegion = %q, want %q (DefaultHomeRegion)", tenant.HomeRegion, DefaultHomeRegion)
	}
	if tenant.EffectiveHomeRegion() != DefaultHomeRegion {
		t.Errorf("EffectiveHomeRegion() = %q, want %q", tenant.EffectiveHomeRegion(), DefaultHomeRegion)
	}
}

func TestNewTenantInRegion(t *testing.T) {
	tests := []struct {
		name       string
		homeRegion string
		wantRegion string
		wantErr    bool
	}{
		{"empty resolves to default", "", DefaultHomeRegion, false},
		{"valid lhr", "lhr", "lhr", false},
		{"valid fra", "fra", "fra", false},
		{"unknown region rejected", "atlantis", "", true},
		{"uppercase rejected (allowlist is case-sensitive)", "IAD", "", true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tenant, err := NewTenantInRegion("acme", "Acme", "", tt.homeRegion)
			if (err != nil) != tt.wantErr {
				t.Fatalf("NewTenantInRegion err = %v, wantErr %v", err, tt.wantErr)
			}
			if tt.wantErr {
				return
			}
			if tenant.HomeRegion != tt.wantRegion {
				t.Errorf("HomeRegion = %q, want %q", tenant.HomeRegion, tt.wantRegion)
			}
		})
	}
}

func TestValidateHomeRegion(t *testing.T) {
	tests := []struct {
		name    string
		region  string
		wantErr bool
	}{
		{"iad allowed", "iad", false},
		{"lhr allowed", "lhr", false},
		{"ord allowed", "ord", false},
		{"fra allowed", "fra", false},
		{"syd allowed", "syd", false},
		{"empty rejected", "", true},
		{"unknown rejected", "atlantis", true},
		{"uppercase rejected", "IAD", true},
		{"mixed-case rejected", "Iad", true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateHomeRegion(tt.region)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateHomeRegion(%q) err = %v, wantErr %v", tt.region, err, tt.wantErr)
			}
		})
	}
}

func TestEffectiveHomeRegion_FallbackForLegacyRows(t *testing.T) {
	// A tenant loaded from storage that pre-dates this field has
	// HomeRegion = "" — EffectiveHomeRegion must paper over that
	// without anyone needing to backfill the metadata column.
	tenant := &Tenant{ID: "legacy", Name: "Legacy", HomeRegion: ""}
	if got := tenant.EffectiveHomeRegion(); got != DefaultHomeRegion {
		t.Errorf("EffectiveHomeRegion() for legacy row = %q, want %q", got, DefaultHomeRegion)
	}
}

func TestTenant_SetHomeRegion(t *testing.T) {
	tenant, err := NewTenant("acme", "Acme", "")
	if err != nil {
		t.Fatal(err)
	}
	originalUpdated := tenant.UpdatedAt

	if err := tenant.SetHomeRegion("lhr"); err != nil {
		t.Fatalf("SetHomeRegion(lhr) returned %v, want nil", err)
	}
	if tenant.HomeRegion != "lhr" {
		t.Errorf("HomeRegion = %q after SetHomeRegion(lhr), want %q", tenant.HomeRegion, "lhr")
	}
	if !tenant.UpdatedAt.After(originalUpdated) {
		t.Errorf("UpdatedAt did not advance: was %v, now %v", originalUpdated, tenant.UpdatedAt)
	}

	if err := tenant.SetHomeRegion("atlantis"); err == nil {
		t.Error("SetHomeRegion(atlantis) returned nil, want error from allowlist check")
	}
	if tenant.HomeRegion != "lhr" {
		t.Errorf("HomeRegion changed to %q after rejected SetHomeRegion call, want %q", tenant.HomeRegion, "lhr")
	}
}
