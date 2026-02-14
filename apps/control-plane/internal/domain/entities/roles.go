package entities

// Role represents a user's role in the system.
type Role string

// Role constants define the possible roles in the system.
const (
	RoleAdmin          Role = "Admin"
	RoleDeveloper      Role = "Developer"
	RoleReadOnly       Role = "ReadOnly"
	RoleServiceAccount Role = "ServiceAccount"
)

// Permission represents a specific permission.
type Permission string

// Permission constants define the possible permissions in the system.
const (
	PermissionRead            Permission = "Read"
	PermissionWrite           Permission = "Write"
	PermissionAdmin           Permission = "Admin"
	PermissionMetrics         Permission = "Metrics"
	PermissionManageSchemas   Permission = "ManageSchemas"
	PermissionManagePipelines Permission = "ManagePipelines"
	PermissionManageTenants   Permission = "ManageTenants"
)
