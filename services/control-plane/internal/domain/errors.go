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

	// ErrUnauthorized is returned when a user is not authorized
	ErrUnauthorized = errors.New("unauthorized")

	// ErrForbidden is returned when an action is forbidden
	ErrForbidden = errors.New("forbidden")

	// ErrInvalidInput is returned when input validation fails
	ErrInvalidInput = errors.New("invalid input")
)
