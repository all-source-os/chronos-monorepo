package entities

import (
	"testing"
)

func TestNewUser(t *testing.T) {
	tests := []struct {
		name     string
		id       string
		username string
		tenantID string
		role     Role
		wantErr  bool
	}{
		{
			name:     "Valid user",
			id:       "user-1",
			username: "testuser",
			tenantID: "tenant-1",
			role:     RoleDeveloper,
			wantErr:  false,
		},
		{
			name:     "Empty ID",
			id:       "",
			username: "testuser",
			tenantID: "tenant-1",
			role:     RoleDeveloper,
			wantErr:  true,
		},
		{
			name:     "Invalid role",
			id:       "user-1",
			username: "testuser",
			tenantID: "tenant-1",
			role:     Role("InvalidRole"),
			wantErr:  true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			user, err := NewUser(tt.id, tt.username, tt.tenantID, tt.role)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewUser() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr {
				if user.ID != tt.id {
					t.Errorf("User.ID = %v, want %v", user.ID, tt.id)
				}
				if user.Role != tt.role {
					t.Errorf("User.Role = %v, want %v", user.Role, tt.role)
				}
			}
		})
	}
}

func TestUser_HasPermission(t *testing.T) {
	tests := []struct {
		name       string
		role       Role
		permission Permission
		want       bool
	}{
		// Admin tests
		{"Admin has Read", RoleAdmin, PermissionRead, true},
		{"Admin has Write", RoleAdmin, PermissionWrite, true},
		{"Admin has Admin", RoleAdmin, PermissionAdmin, true},
		{"Admin has ManageTenants", RoleAdmin, PermissionManageTenants, true},

		// Developer tests
		{"Developer has Read", RoleDeveloper, PermissionRead, true},
		{"Developer has Write", RoleDeveloper, PermissionWrite, true},
		{"Developer no Admin", RoleDeveloper, PermissionAdmin, false},
		{"Developer no ManageTenants", RoleDeveloper, PermissionManageTenants, false},

		// ReadOnly tests
		{"ReadOnly has Read", RoleReadOnly, PermissionRead, true},
		{"ReadOnly no Write", RoleReadOnly, PermissionWrite, false},
		{"ReadOnly has Metrics", RoleReadOnly, PermissionMetrics, true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			user, _ := NewUser("user-1", "test", "tenant-1", tt.role)
			if got := user.HasPermission(tt.permission); got != tt.want {
				t.Errorf("User.HasPermission() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestUser_BelongsToTenant(t *testing.T) {
	user, _ := NewUser("user-1", "test", "tenant-1", RoleDeveloper)

	if !user.BelongsToTenant("tenant-1") {
		t.Error("User should belong to tenant-1")
	}

	if user.BelongsToTenant("tenant-2") {
		t.Error("User should not belong to tenant-2")
	}
}
