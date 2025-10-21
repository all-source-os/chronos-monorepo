package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sync"
	"time"

	"github.com/gin-gonic/gin"
)

// AuditEvent represents an auditable action
type AuditEvent struct {
	Timestamp   time.Time          `json:"timestamp"`
	EventType   string             `json:"event_type"`
	UserID      string             `json:"user_id,omitempty"`
	Username    string             `json:"username,omitempty"`
	TenantID    string             `json:"tenant_id,omitempty"`
	Action      string             `json:"action"`
	Resource    string             `json:"resource,omitempty"`
	ResourceID  string             `json:"resource_id,omitempty"`
	Method      string             `json:"method"`
	Path        string             `json:"path"`
	StatusCode  int                `json:"status_code"`
	Duration    float64            `json:"duration_ms"`
	IPAddress   string             `json:"ip_address,omitempty"`
	UserAgent   string             `json:"user_agent,omitempty"`
	Error       string             `json:"error,omitempty"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
}

// AuditLogger handles audit logging
type AuditLogger struct {
	file   *os.File
	mu     sync.Mutex
	enabled bool
}

// NewAuditLogger creates a new audit logger
func NewAuditLogger(filePath string) (*AuditLogger, error) {
	if filePath == "" {
		// Audit logging disabled
		return &AuditLogger{enabled: false}, nil
	}

	file, err := os.OpenFile(filePath, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return nil, fmt.Errorf("failed to open audit log file: %w", err)
	}

	return &AuditLogger{
		file:   file,
		enabled: true,
	}, nil
}

// Log writes an audit event to the log file
func (a *AuditLogger) Log(event AuditEvent) error {
	if !a.enabled {
		return nil
	}

	a.mu.Lock()
	defer a.mu.Unlock()

	event.Timestamp = time.Now().UTC()

	data, err := json.Marshal(event)
	if err != nil {
		return fmt.Errorf("failed to marshal audit event: %w", err)
	}

	if _, err := a.file.Write(append(data, '\n')); err != nil {
		return fmt.Errorf("failed to write audit event: %w", err)
	}

	return nil
}

// Close closes the audit log file
func (a *AuditLogger) Close() error {
	if a.file != nil {
		return a.file.Close()
	}
	return nil
}

// AuditMiddleware logs all requests for audit purposes
func AuditMiddleware(logger *AuditLogger) gin.HandlerFunc {
	return func(c *gin.Context) {
		// Skip health and metrics endpoints
		if c.Request.URL.Path == "/health" || c.Request.URL.Path == "/metrics" {
			c.Next()
			return
		}

		start := time.Now()

		// Process request
		c.Next()

		// Calculate duration
		duration := time.Since(start).Milliseconds()

		// Extract auth context if available
		var userID, username, tenantID string
		if auth, exists := c.Get("auth"); exists {
			if authCtx, ok := auth.(*AuthContext); ok {
				userID = authCtx.UserID
				username = authCtx.Username
				tenantID = authCtx.TenantID
			}
		}

		// Determine action from method and path
		action := determineAction(c.Request.Method, c.Request.URL.Path)

		// Create audit event
		event := AuditEvent{
			EventType:  "api_request",
			UserID:     userID,
			Username:   username,
			TenantID:   tenantID,
			Action:     action,
			Resource:   extractResource(c.Request.URL.Path),
			Method:     c.Request.Method,
			Path:       c.Request.URL.Path,
			StatusCode: c.Writer.Status(),
			Duration:   float64(duration),
			IPAddress:  c.ClientIP(),
			UserAgent:  c.Request.UserAgent(),
		}

		// Add error if request failed
		if c.Writer.Status() >= 400 {
			if errors := c.Errors.ByType(gin.ErrorTypeAny); len(errors) > 0 {
				event.Error = errors.String()
			}
		}

		// Log the event
		if err := logger.Log(event); err != nil {
			// Log to stderr if audit logging fails (don't fail the request)
			fmt.Fprintf(os.Stderr, "Failed to write audit log: %v\n", err)
		}
	}
}

// determineAction extracts a human-readable action from method and path
func determineAction(method, path string) string {
	switch method {
	case "GET":
		return "read"
	case "POST":
		if contains(path, "login") {
			return "login"
		}
		if contains(path, "register") {
			return "register"
		}
		return "create"
	case "PUT":
		return "update"
	case "DELETE":
		return "delete"
	case "PATCH":
		return "modify"
	default:
		return method
	}
}

// extractResource extracts the resource type from the path
func extractResource(path string) string {
	if contains(path, "/tenants") {
		return "tenant"
	}
	if contains(path, "/users") {
		return "user"
	}
	if contains(path, "/snapshots") {
		return "snapshot"
	}
	if contains(path, "/backups") {
		return "backup"
	}
	if contains(path, "/cluster") {
		return "cluster"
	}
	if contains(path, "/operations") {
		return "operation"
	}
	return "unknown"
}

// contains checks if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || indexOf(s, substr) >= 0)
}

// indexOf returns the index of substr in s, or -1 if not found
func indexOf(s, substr string) int {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return i
		}
	}
	return -1
}

// LogAuthEvent logs an authentication-related event
func (a *AuditLogger) LogAuthEvent(eventType, userID, username, tenantID, details string) {
	event := AuditEvent{
		EventType: eventType,
		UserID:    userID,
		Username:  username,
		TenantID:  tenantID,
		Action:    eventType,
		Metadata: map[string]interface{}{
			"details": details,
		},
	}
	a.Log(event)
}

// LogTenantEvent logs a tenant management event
func (a *AuditLogger) LogTenantEvent(action, tenantID, userID, details string) {
	event := AuditEvent{
		EventType:  "tenant_management",
		Action:     action,
		Resource:   "tenant",
		ResourceID: tenantID,
		UserID:     userID,
		Metadata: map[string]interface{}{
			"details": details,
		},
	}
	a.Log(event)
}

// LogOperationEvent logs an operational event (snapshot, backup, etc.)
func (a *AuditLogger) LogOperationEvent(operation, resourceID, userID, status string) {
	event := AuditEvent{
		EventType:  "operation",
		Action:     operation,
		Resource:   operation,
		ResourceID: resourceID,
		UserID:     userID,
		Metadata: map[string]interface{}{
			"status": status,
		},
	}
	a.Log(event)
}
