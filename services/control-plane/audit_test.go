package main

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

func TestAuditLogger_Log(t *testing.T) {
	// Create a temporary audit log file
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Create a test event
	event := AuditEvent{
		EventType:  "test_event",
		UserID:     "user-123",
		Username:   "testuser",
		TenantID:   "tenant-456",
		Action:     "test_action",
		Resource:   "test_resource",
		Method:     "GET",
		Path:       "/test/path",
		StatusCode: 200,
		Duration:   123.45,
		IPAddress:  "192.168.1.1",
		UserAgent:  "TestAgent/1.0",
	}

	// Log the event
	err = logger.Log(event)
	if err != nil {
		t.Fatalf("Failed to log event: %v", err)
	}

	// Close logger to flush
	logger.Close()

	// Read the log file
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	// Verify the event was logged
	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	if len(lines) != 1 {
		t.Errorf("Expected 1 log line, got %d", len(lines))
	}

	// Parse the logged event
	var logged AuditEvent
	err = json.Unmarshal([]byte(lines[0]), &logged)
	if err != nil {
		t.Fatalf("Failed to parse logged event: %v", err)
	}

	// Verify fields
	if logged.EventType != event.EventType {
		t.Errorf("EventType mismatch: expected %s, got %s", event.EventType, logged.EventType)
	}
	if logged.UserID != event.UserID {
		t.Errorf("UserID mismatch: expected %s, got %s", event.UserID, logged.UserID)
	}
	if logged.TenantID != event.TenantID {
		t.Errorf("TenantID mismatch: expected %s, got %s", event.TenantID, logged.TenantID)
	}
	if logged.StatusCode != event.StatusCode {
		t.Errorf("StatusCode mismatch: expected %d, got %d", event.StatusCode, logged.StatusCode)
	}

	// Verify timestamp was set
	if logged.Timestamp.IsZero() {
		t.Error("Timestamp should have been set automatically")
	}
}

func TestAuditLogger_MultipleEvents(t *testing.T) {
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Log multiple events
	numEvents := 10
	for i := 0; i < numEvents; i++ {
		event := AuditEvent{
			EventType: "test_event",
			UserID:    "user-123",
			Action:    "test_action",
		}
		err := logger.Log(event)
		if err != nil {
			t.Errorf("Failed to log event %d: %v", i, err)
		}
	}

	logger.Close()

	// Read and verify
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	if len(lines) != numEvents {
		t.Errorf("Expected %d log lines, got %d", numEvents, len(lines))
	}
}

func TestAuditLogger_LogAuthEvent(t *testing.T) {
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Log an auth event
	logger.LogAuthEvent("login", "user-123", "testuser", "tenant-456", "successful login")

	logger.Close()

	// Verify
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	var logged AuditEvent
	err = json.Unmarshal(content, &logged)
	if err != nil {
		t.Fatalf("Failed to parse logged event: %v", err)
	}

	if logged.EventType != "login" {
		t.Errorf("EventType mismatch: expected 'login', got '%s'", logged.EventType)
	}
	if logged.UserID != "user-123" {
		t.Errorf("UserID mismatch: expected 'user-123', got '%s'", logged.UserID)
	}
}

func TestAuditLogger_LogTenantEvent(t *testing.T) {
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Log a tenant event
	logger.LogTenantEvent("create", "tenant-789", "user-123", "new tenant created")

	logger.Close()

	// Verify
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	var logged AuditEvent
	err = json.Unmarshal(content, &logged)
	if err != nil {
		t.Fatalf("Failed to parse logged event: %v", err)
	}

	if logged.EventType != "tenant_management" {
		t.Errorf("EventType mismatch: expected 'tenant_management', got '%s'", logged.EventType)
	}
	if logged.ResourceID != "tenant-789" {
		t.Errorf("ResourceID mismatch: expected 'tenant-789', got '%s'", logged.ResourceID)
	}
	if logged.Action != "create" {
		t.Errorf("Action mismatch: expected 'create', got '%s'", logged.Action)
	}
}

func TestAuditLogger_LogOperationEvent(t *testing.T) {
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Log an operation event
	logger.LogOperationEvent("snapshot", "snapshot-123", "user-456", "success")

	logger.Close()

	// Verify
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	var logged AuditEvent
	err = json.Unmarshal(content, &logged)
	if err != nil {
		t.Fatalf("Failed to parse logged event: %v", err)
	}

	if logged.EventType != "operation" {
		t.Errorf("EventType mismatch: expected 'operation', got '%s'", logged.EventType)
	}
	if logged.Action != "snapshot" {
		t.Errorf("Action mismatch: expected 'snapshot', got '%s'", logged.Action)
	}
	if logged.ResourceID != "snapshot-123" {
		t.Errorf("ResourceID mismatch: expected 'snapshot-123', got '%s'", logged.ResourceID)
	}
}

func TestAuditLogger_Disabled(t *testing.T) {
	// Create logger with empty path (disabled)
	logger, err := NewAuditLogger("")
	if err != nil {
		t.Fatalf("Failed to create disabled logger: %v", err)
	}

	if logger.enabled {
		t.Error("Logger should be disabled when path is empty")
	}

	// Logging should succeed but do nothing
	event := AuditEvent{
		EventType: "test",
		UserID:    "user-123",
	}

	err = logger.Log(event)
	if err != nil {
		t.Errorf("Logging to disabled logger should not error: %v", err)
	}
}

func TestDetermineAction(t *testing.T) {
	tests := []struct {
		method   string
		path     string
		expected string
	}{
		{"GET", "/api/v1/users", "read"},
		{"POST", "/api/v1/users", "create"},
		{"POST", "/api/v1/auth/login", "login"},
		{"POST", "/api/v1/auth/register", "register"},
		{"PUT", "/api/v1/tenants/123", "update"},
		{"DELETE", "/api/v1/users/456", "delete"},
		{"PATCH", "/api/v1/settings", "modify"},
	}

	for _, tt := range tests {
		t.Run(tt.method+"_"+tt.path, func(t *testing.T) {
			action := determineAction(tt.method, tt.path)
			if action != tt.expected {
				t.Errorf("Expected action '%s', got '%s'", tt.expected, action)
			}
		})
	}
}

func TestExtractResource(t *testing.T) {
	tests := []struct {
		path     string
		expected string
	}{
		{"/api/v1/tenants", "tenant"},
		{"/api/v1/tenants/123", "tenant"},
		{"/api/v1/users", "user"},
		{"/api/v1/users/456", "user"},
		{"/api/v1/operations/snapshot", "operation"},
		{"/api/v1/backups", "backup"},
		{"/api/v1/snapshots", "snapshot"},
		{"/api/v1/cluster/status", "cluster"},
		{"/api/v1/unknown/resource", "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.path, func(t *testing.T) {
			resource := extractResource(tt.path)
			if resource != tt.expected {
				t.Errorf("Expected resource '%s', got '%s'", tt.expected, resource)
			}
		})
	}
}

func TestAuditLogger_Concurrency(t *testing.T) {
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpfile.Name())
	defer tmpfile.Close()

	logger, err := NewAuditLogger(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}
	defer logger.Close()

	// Log events concurrently
	numGoroutines := 10
	eventsPerGoroutine := 10
	done := make(chan bool)

	for i := 0; i < numGoroutines; i++ {
		go func(id int) {
			for j := 0; j < eventsPerGoroutine; j++ {
				event := AuditEvent{
					EventType: "concurrent_test",
					UserID:    "user-123",
					Action:    "test",
				}
				logger.Log(event)
			}
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < numGoroutines; i++ {
		<-done
	}

	logger.Close()

	// Verify all events were logged
	content, err := os.ReadFile(tmpfile.Name())
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	expectedLines := numGoroutines * eventsPerGoroutine
	if len(lines) != expectedLines {
		t.Errorf("Expected %d log lines, got %d", expectedLines, len(lines))
	}
}
