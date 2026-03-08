// Package domain contains domain errors and business logic.
package domain

import "errors"

var (
	// ErrTenantAlreadyExists is returned when attempting to create a tenant that already exists
	ErrTenantAlreadyExists = errors.New("tenant already exists")

	// ErrTenantNotFound is returned when a tenant is not found
	ErrTenantNotFound = errors.New("tenant not found")

	// ErrUserAlreadyExists is returned when attempting to create a user that already exists
	ErrUserAlreadyExists = errors.New("user already exists")

	// ErrUserNotFound is returned when a user is not found
	ErrUserNotFound = errors.New("user not found")

	// ErrPolicyAlreadyExists is returned when attempting to create a policy that already exists
	ErrPolicyAlreadyExists = errors.New("policy already exists")

	// ErrPolicyNotFound is returned when a policy is not found
	ErrPolicyNotFound = errors.New("policy not found")

	// ErrOperationNotFound is returned when an operation is not found
	ErrOperationNotFound = errors.New("operation not found")

	// ErrUnauthorized is returned when a user is not authorized
	ErrUnauthorized = errors.New("unauthorized")

	// ErrForbidden is returned when an action is forbidden
	ErrForbidden = errors.New("forbidden")

	// ErrInvalidInput is returned when input validation fails
	ErrInvalidInput = errors.New("invalid input")

	// ErrConfigNotFound is returned when a config entry is not found
	ErrConfigNotFound = errors.New("config entry not found")

	// ErrConfigAlreadyExists is returned when a config entry already exists
	ErrConfigAlreadyExists = errors.New("config entry already exists")

	// ErrCoreNotAvailable is returned when the Core service is not configured
	ErrCoreNotAvailable = errors.New("core service not available")

	// ErrAlertRuleNotFound is returned when an alert rule is not found
	ErrAlertRuleNotFound = errors.New("alert rule not found")

	// ErrAlertRuleAlreadyExists is returned when an alert rule already exists
	ErrAlertRuleAlreadyExists = errors.New("alert rule already exists")

	// ErrSLONotFound is returned when an SLO is not found
	ErrSLONotFound = errors.New("SLO not found")

	// ErrSLOAlreadyExists is returned when an SLO already exists
	ErrSLOAlreadyExists = errors.New("SLO already exists")

	// ErrIPRuleNotFound is returned when an IP rule is not found
	ErrIPRuleNotFound = errors.New("IP rule not found")

	// ErrIPRuleAlreadyExists is returned when an IP rule already exists
	ErrIPRuleAlreadyExists = errors.New("IP rule already exists")
)
