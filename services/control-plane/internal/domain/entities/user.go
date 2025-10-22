package entities

import (
	"errors"
	"time"
)

// User represents a user in the system
type User struct {
	ID        string
	Username  string
	TenantID  string
	Role      Role
	IsAPIKey  bool
	CreatedAt time.Time
	UpdatedAt time.Time
}

// Role represents a user's role in the system
type Role string

const (
	RoleAdmin          Role = "Admin"
	RoleDeveloper      Role = "Developer"
	RoleReadOnly       Role = "ReadOnly"
	RoleServiceAccount Role = "ServiceAccount"
)

// Permission represents a specific permission
type Permission string

const (
	PermissionRead            Permission = "Read"
	PermissionWrite           Permission = "Write"
	PermissionAdmin           Permission = "Admin"
	PermissionMetrics         Permission = "Metrics"
	PermissionManageSchemas   Permission = "ManageSchemas"
	PermissionManagePipelines Permission = "ManagePipelines"
	PermissionManageTenants   Permission = "ManageTenants"
)

// NewUser creates a new user with validation
func NewUser(id, username, tenantID string, role Role) (*User, error) {
	if err := ValidateUserID(id); err != nil {
		return nil, err
	}
	if err := ValidateUsername(username); err != nil {
		return nil, err
	}
	if err := ValidateTenantID(tenantID); err != nil {
		return nil, err
	}
	if err := ValidateRole(role); err != nil {
		return nil, err
	}

	now := time.Now()
	return &User{
		ID:        id,
		Username:  username,
		TenantID:  tenantID,
		Role:      role,
		IsAPIKey:  false,
		CreatedAt: now,
		UpdatedAt: now,
	}, nil
}

// ValidateUserID validates a user ID
func ValidateUserID(id string) error {
	if id == "" {
		return errors.New("user ID cannot be empty")
	}
	return nil
}

// ValidateUsername validates a username
func ValidateUsername(username string) error {
	if username == "" {
		return errors.New("username cannot be empty")
	}
	if len(username) > 255 {
		return errors.New("username too long")
	}
	return nil
}

// ValidateRole validates a role
func ValidateRole(role Role) error {
	switch role {
	case RoleAdmin, RoleDeveloper, RoleReadOnly, RoleServiceAccount:
		return nil
	default:
		return errors.New("invalid role")
	}
}

// HasPermission checks if user has a specific permission based on role
func (u *User) HasPermission(perm Permission) bool {
	switch u.Role {
	case RoleAdmin:
		return true // Admin has all permissions
	case RoleDeveloper:
		return perm == PermissionRead || perm == PermissionWrite || perm == PermissionMetrics || perm == PermissionManageSchemas || perm == PermissionManagePipelines
	case RoleReadOnly:
		return perm == PermissionRead || perm == PermissionMetrics
	case RoleServiceAccount:
		return perm == PermissionRead || perm == PermissionWrite
	default:
		return false
	}
}

// IsAdmin checks if user is an admin
func (u *User) IsAdmin() bool {
	return u.Role == RoleAdmin
}

// BelongsToTenant checks if user belongs to a specific tenant
func (u *User) BelongsToTenant(tenantID string) bool {
	return u.TenantID == tenantID
}

// AssignRole assigns a new role to the user
func (u *User) AssignRole(role Role) error {
	if err := ValidateRole(role); err != nil {
		return err
	}
	u.Role = role
	u.UpdatedAt = time.Now()
	return nil
}
