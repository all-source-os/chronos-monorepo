package entities

import (
	"errors"
	"time"
)

// AuditEvent represents an auditable action
type AuditEvent struct {
	Timestamp  time.Time
	EventType  string
	UserID     string
	Username   string
	TenantID   string
	Action     string
	Resource   string
	ResourceID string
	Method     string
	Path       string
	StatusCode int
	Duration   float64
	IPAddress  string
	UserAgent  string
	Error      string
	Metadata   map[string]interface{}
}

// NewAuditEvent creates a new audit event
func NewAuditEvent(eventType, action, method, path string) (*AuditEvent, error) {
	if err := ValidateEventType(eventType); err != nil {
		return nil, err
	}
	if err := ValidateAction(action); err != nil {
		return nil, err
	}

	return &AuditEvent{
		Timestamp: time.Now(),
		EventType: eventType,
		Action:    action,
		Method:    method,
		Path:      path,
		Metadata:  make(map[string]interface{}),
	}, nil
}

// ValidateEventType validates an event type
func ValidateEventType(eventType string) error {
	if eventType == "" {
		return errors.New("event type cannot be empty")
	}
	return nil
}

// ValidateAction validates an action
func ValidateAction(action string) error {
	if action == "" {
		return errors.New("action cannot be empty")
	}
	return nil
}

// WithUser adds user information to the audit event
func (ae *AuditEvent) WithUser(userID, username string) *AuditEvent {
	ae.UserID = userID
	ae.Username = username
	return ae
}

// WithTenant adds tenant information to the audit event
func (ae *AuditEvent) WithTenant(tenantID string) *AuditEvent {
	ae.TenantID = tenantID
	return ae
}

// WithResource adds resource information to the audit event
func (ae *AuditEvent) WithResource(resource, resourceID string) *AuditEvent {
	ae.Resource = resource
	ae.ResourceID = resourceID
	return ae
}

// WithStatus adds status information to the audit event
func (ae *AuditEvent) WithStatus(statusCode int, duration float64) *AuditEvent {
	ae.StatusCode = statusCode
	ae.Duration = duration
	return ae
}

// WithClient adds client information to the audit event
func (ae *AuditEvent) WithClient(ipAddress, userAgent string) *AuditEvent {
	ae.IPAddress = ipAddress
	ae.UserAgent = userAgent
	return ae
}

// WithError adds error information to the audit event
func (ae *AuditEvent) WithError(err string) *AuditEvent {
	ae.Error = err
	return ae
}

// AddMetadata adds metadata to the audit event
func (ae *AuditEvent) AddMetadata(key string, value interface{}) *AuditEvent {
	ae.Metadata[key] = value
	return ae
}

// IsError checks if the audit event represents an error
func (ae *AuditEvent) IsError() bool {
	return ae.StatusCode >= 400 || ae.Error != ""
}
