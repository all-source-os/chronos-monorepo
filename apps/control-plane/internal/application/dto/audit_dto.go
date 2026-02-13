package dto

import "time"

// AuditQueryRequest represents a request to query audit events
type AuditQueryRequest struct {
	UserID   string `form:"user_id"`
	TenantID string `form:"tenant_id"`
	Start    string `form:"start"`
	End      string `form:"end"`
	Errors   bool   `form:"errors"`
	Limit    int    `form:"limit"`
}

// AuditEventResponse represents an audit event in API responses
type AuditEventResponse struct {
	Timestamp  time.Time      `json:"timestamp"`
	EventType  string         `json:"event_type"`
	UserID     string         `json:"user_id,omitempty"`
	Username   string         `json:"username,omitempty"`
	TenantID   string         `json:"tenant_id,omitempty"`
	Action     string         `json:"action"`
	Resource   string         `json:"resource,omitempty"`
	ResourceID string         `json:"resource_id,omitempty"`
	Method     string         `json:"method"`
	Path       string         `json:"path"`
	StatusCode int            `json:"status_code"`
	Duration   float64        `json:"duration_ms"`
	IPAddress  string         `json:"ip_address,omitempty"`
	Error      string         `json:"error,omitempty"`
	Metadata   map[string]any `json:"metadata,omitempty"`
}

// SchemaGovernanceRequest represents a request to register or update a schema
type SchemaGovernanceRequest struct {
	EventType     string `json:"event_type" binding:"required"`
	Schema        any    `json:"schema" binding:"required"`
	Version       string `json:"version"`
	Compatibility string `json:"compatibility"`
}

// SchemaGovernanceResponse represents a schema governance response
type SchemaGovernanceResponse struct {
	EventType string `json:"event_type"`
	Schema    any    `json:"schema"`
	Version   string `json:"version"`
	Status    string `json:"status"`
}

// ValidateEventRequest represents a request to validate an event against a schema
type ValidateEventRequest struct {
	EventType string `json:"event_type" binding:"required"`
	Data      any    `json:"data" binding:"required"`
}

// ValidateEventResponse represents a validation response
type ValidateEventResponse struct {
	Valid  bool     `json:"valid"`
	Errors []string `json:"errors,omitempty"`
}
