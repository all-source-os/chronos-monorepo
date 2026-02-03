package main

import (
	"encoding/json"
	"os"
	"strings"
	"sync"
	"testing"
	"time"
)

// testLoggerSetup creates a temporary logger for testing and returns cleanup function
func testLoggerSetup(t *testing.T) (logger *AuditLogger, tmpfileName string) {
	t.Helper()
	tmpfile, err := os.CreateTemp("", "audit-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	tmpfileName = tmpfile.Name()
	_ = tmpfile.Close() //nolint:errcheck // test cleanup

	t.Cleanup(func() {
		_ = os.Remove(tmpfileName) //nolint:errcheck // test cleanup
	})

	logger, err = NewAuditLogger(tmpfileName)
	if err != nil {
		t.Fatalf("Failed to create audit logger: %v", err)
	}

	t.Cleanup(func() {
		_ = logger.Close() //nolint:errcheck // test cleanup
	})

	return logger, tmpfileName
}

func TestAuditLogger_Log(t *testing.T) {
	logger, tmpfileName := testLoggerSetup(t)

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
	err := logger.Log(event)
	if err != nil {
		t.Fatalf("Failed to log event: %v", err)
	}

	// Close logger to flush
	_ = logger.Close() //nolint:errcheck // test cleanup

	// Read the log file
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
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
	logger, tmpfileName := testLoggerSetup(t)

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

	_ = logger.Close() //nolint:errcheck // test cleanup

	// Read and verify
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	if len(lines) != numEvents {
		t.Errorf("Expected %d log lines, got %d", numEvents, len(lines))
	}
}

// testLogAndVerify is a helper that logs an event and verifies its content
func testLogAndVerify(t *testing.T, logger *AuditLogger, tmpfileName string, logFunc func(), verify func(*AuditEvent)) {
	t.Helper()

	logFunc()
	_ = logger.Close() //nolint:errcheck // test cleanup

	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	var logged AuditEvent
	if err := json.Unmarshal(content, &logged); err != nil {
		t.Fatalf("Failed to parse logged event: %v", err)
	}

	verify(&logged)
}

func TestAuditLogger_LogAuthEvent(t *testing.T) {
	logger, tmpfileName := testLoggerSetup(t)

	testLogAndVerify(t, logger, tmpfileName,
		func() {
			logger.LogAuthEvent("login", "user-123", "testuser", "tenant-456", "successful login")
		},
		func(logged *AuditEvent) {
			if logged.EventType != "login" {
				t.Errorf("EventType mismatch: expected 'login', got '%s'", logged.EventType)
			}
			if logged.UserID != "user-123" {
				t.Errorf("UserID mismatch: expected 'user-123', got '%s'", logged.UserID)
			}
		},
	)
}

func TestAuditLogger_LogTenantEvent(t *testing.T) {
	logger, tmpfileName := testLoggerSetup(t)

	testLogAndVerify(t, logger, tmpfileName,
		func() {
			logger.LogTenantEvent("create", "tenant-789", "user-123", "new tenant created")
		},
		func(logged *AuditEvent) {
			if logged.EventType != "tenant_management" {
				t.Errorf("EventType mismatch: expected 'tenant_management', got '%s'", logged.EventType)
			}
			if logged.ResourceID != "tenant-789" {
				t.Errorf("ResourceID mismatch: expected 'tenant-789', got '%s'", logged.ResourceID)
			}
			if logged.Action != "create" {
				t.Errorf("Action mismatch: expected 'create', got '%s'", logged.Action)
			}
		},
	)
}

func TestAuditLogger_LogOperationEvent(t *testing.T) {
	logger, tmpfileName := testLoggerSetup(t)

	testLogAndVerify(t, logger, tmpfileName,
		func() {
			logger.LogOperationEvent("snapshot", "snapshot-123", "user-456", "success")
		},
		func(logged *AuditEvent) {
			if logged.EventType != "operation" {
				t.Errorf("EventType mismatch: expected 'operation', got '%s'", logged.EventType)
			}
			if logged.Action != "snapshot" {
				t.Errorf("Action mismatch: expected 'snapshot', got '%s'", logged.Action)
			}
			if logged.ResourceID != "snapshot-123" {
				t.Errorf("ResourceID mismatch: expected 'snapshot-123', got '%s'", logged.ResourceID)
			}
		},
	)
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
	logger, tmpfileName := testLoggerSetup(t)

	// Log events concurrently
	numGoroutines := 10
	eventsPerGoroutine := 10
	done := make(chan bool)

	for i := 0; i < numGoroutines; i++ {
		go func() {
			for j := 0; j < eventsPerGoroutine; j++ {
				event := AuditEvent{
					EventType: "concurrent_test",
					UserID:    "user-123",
					Action:    "test",
				}
				_ = logger.Log(event) //nolint:errcheck // concurrent test
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < numGoroutines; i++ {
		<-done
	}

	_ = logger.Close() //nolint:errcheck // test cleanup

	// Verify all events were logged
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	expectedLines := numGoroutines * eventsPerGoroutine
	if len(lines) != expectedLines {
		t.Errorf("Expected %d log lines, got %d", expectedLines, len(lines))
	}
}

// testAsyncLoggerSetup creates a temporary async logger for testing
func testAsyncLoggerSetup(t *testing.T) (logger *AuditLogger, tmpfileName string) {
	t.Helper()
	tmpfile, err := os.CreateTemp("", "audit-async-test-*.log")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	tmpfileName = tmpfile.Name()
	_ = tmpfile.Close() //nolint:errcheck // test cleanup

	t.Cleanup(func() {
		_ = os.Remove(tmpfileName) //nolint:errcheck // test cleanup
	})

	logger, err = NewAsyncAuditLogger(tmpfileName)
	if err != nil {
		t.Fatalf("Failed to create async audit logger: %v", err)
	}

	t.Cleanup(func() {
		_ = logger.Close() //nolint:errcheck // test cleanup
	})

	return logger, tmpfileName
}

func TestAsyncAuditLogger_Log(t *testing.T) {
	logger, tmpfileName := testAsyncLoggerSetup(t)

	// Create a test event
	event := AuditEvent{
		EventType:  "async_test_event",
		UserID:     "user-123",
		Username:   "testuser",
		TenantID:   "tenant-456",
		Action:     "test_action",
		Resource:   "test_resource",
		Method:     "GET",
		Path:       "/test/path",
		StatusCode: 200,
		Duration:   123.45,
	}

	// Log the event
	err := logger.Log(event)
	if err != nil {
		t.Fatalf("Failed to log event: %v", err)
	}

	// Close logger to flush all events
	_ = logger.Close() //nolint:errcheck // test cleanup

	// Read the log file
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
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

	if logged.EventType != event.EventType {
		t.Errorf("EventType mismatch: expected %s, got %s", event.EventType, logged.EventType)
	}
}

func TestAsyncAuditLogger_HighVolume(t *testing.T) {
	logger, tmpfileName := testAsyncLoggerSetup(t)

	// Log many events quickly
	numEvents := 1000
	for i := 0; i < numEvents; i++ {
		event := AuditEvent{
			EventType: "high_volume_test",
			UserID:    "user-123",
			Action:    "test",
		}
		_ = logger.Log(event) //nolint:errcheck // high volume test
	}

	// Close to flush all events
	_ = logger.Close() //nolint:errcheck // test cleanup

	// Read and verify
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	if len(lines) != numEvents {
		t.Errorf("Expected %d log lines, got %d", numEvents, len(lines))
	}
}

func TestAsyncAuditLogger_ConcurrentWrites(t *testing.T) {
	logger, tmpfileName := testAsyncLoggerSetup(t)

	// Log events concurrently from multiple goroutines
	numGoroutines := 50
	eventsPerGoroutine := 100
	var wg sync.WaitGroup

	for i := 0; i < numGoroutines; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := 0; j < eventsPerGoroutine; j++ {
				event := AuditEvent{
					EventType: "concurrent_async_test",
					UserID:    "user-123",
					Action:    "test",
				}
				_ = logger.Log(event) //nolint:errcheck // concurrent test
			}
		}()
	}

	wg.Wait()
	_ = logger.Close() //nolint:errcheck // test cleanup

	// Verify all events were logged
	content, err := os.ReadFile(tmpfileName) //nolint:gosec // test file path
	if err != nil {
		t.Fatalf("Failed to read log file: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(content)), "\n")
	expectedLines := numGoroutines * eventsPerGoroutine
	if len(lines) != expectedLines {
		t.Errorf("Expected %d log lines, got %d", expectedLines, len(lines))
	}
}

func TestAsyncAuditLogger_Disabled(t *testing.T) {
	// Create logger with empty path (disabled)
	logger, err := NewAsyncAuditLogger("")
	if err != nil {
		t.Fatalf("Failed to create disabled logger: %v", err)
	}

	if logger.enabled {
		t.Error("Logger should be disabled when path is empty")
	}

	if logger.asyncEnabled {
		t.Error("Async should be disabled when path is empty")
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

// Benchmark sync vs async audit logging

func BenchmarkAuditLogger_Sync(b *testing.B) {
	tmpfile, err := os.CreateTemp("", "audit-bench-sync-*.log")
	if err != nil {
		b.Fatalf("Failed to create temp file: %v", err)
	}
	tmpfileName := tmpfile.Name()
	_ = tmpfile.Close()                           //nolint:errcheck // benchmark cleanup
	defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark cleanup

	logger, err := NewAuditLogger(tmpfileName)
	if err != nil {
		b.Fatalf("Failed to create audit logger: %v", err)
	}
	defer func() { _ = logger.Close() }() //nolint:errcheck // benchmark cleanup

	event := AuditEvent{
		EventType:  "benchmark_event",
		UserID:     "user-123",
		Username:   "testuser",
		TenantID:   "tenant-456",
		Action:     "read",
		Resource:   "article",
		Method:     "GET",
		Path:       "/api/v1/articles/123",
		StatusCode: 200,
		Duration:   10.5,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = logger.Log(event) //nolint:errcheck // benchmark
	}
}

func BenchmarkAuditLogger_Async(b *testing.B) {
	tmpfile, err := os.CreateTemp("", "audit-bench-async-*.log")
	if err != nil {
		b.Fatalf("Failed to create temp file: %v", err)
	}
	tmpfileName := tmpfile.Name()
	_ = tmpfile.Close()                           //nolint:errcheck // benchmark cleanup
	defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark cleanup

	logger, err := NewAsyncAuditLogger(tmpfileName)
	if err != nil {
		b.Fatalf("Failed to create async audit logger: %v", err)
	}
	defer func() { _ = logger.Close() }() //nolint:errcheck // benchmark cleanup

	event := AuditEvent{
		EventType:  "benchmark_event",
		UserID:     "user-123",
		Username:   "testuser",
		TenantID:   "tenant-456",
		Action:     "read",
		Resource:   "article",
		Method:     "GET",
		Path:       "/api/v1/articles/123",
		StatusCode: 200,
		Duration:   10.5,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = logger.Log(event) //nolint:errcheck // benchmark
	}
}

func BenchmarkAuditLogger_Async_Parallel(b *testing.B) {
	tmpfile, err := os.CreateTemp("", "audit-bench-async-parallel-*.log")
	if err != nil {
		b.Fatalf("Failed to create temp file: %v", err)
	}
	tmpfileName := tmpfile.Name()
	_ = tmpfile.Close()                           //nolint:errcheck // benchmark cleanup
	defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark cleanup

	logger, err := NewAsyncAuditLogger(tmpfileName)
	if err != nil {
		b.Fatalf("Failed to create async audit logger: %v", err)
	}
	defer func() { _ = logger.Close() }() //nolint:errcheck // benchmark cleanup

	event := AuditEvent{
		EventType:  "benchmark_event",
		UserID:     "user-123",
		Username:   "testuser",
		TenantID:   "tenant-456",
		Action:     "read",
		Resource:   "article",
		Method:     "GET",
		Path:       "/api/v1/articles/123",
		StatusCode: 200,
		Duration:   10.5,
	}

	b.ResetTimer()
	b.RunParallel(func(pb *testing.PB) {
		for pb.Next() {
			_ = logger.Log(event) //nolint:errcheck // benchmark
		}
	})
}

// BenchmarkAuditLogger_LatencyComparison compares sync vs async latency in a realistic scenario
func BenchmarkAuditLogger_LatencyComparison(b *testing.B) {
	b.Run("Sync_SingleEvent", func(b *testing.B) {
		tmpfile, _ := os.CreateTemp("", "audit-latency-sync-*.log") //nolint:errcheck // benchmark
		tmpfileName := tmpfile.Name()
		_ = tmpfile.Close()                           //nolint:errcheck // benchmark
		defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark

		logger, _ := NewAuditLogger(tmpfileName) //nolint:errcheck // benchmark
		defer func() { _ = logger.Close() }()    //nolint:errcheck // benchmark

		event := AuditEvent{EventType: "test", UserID: "user-123", Action: "read"}

		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			start := time.Now()
			_ = logger.Log(event) //nolint:errcheck // benchmark
			_ = time.Since(start) // Measure latency (would be recorded in real profiling)
		}
	})

	b.Run("Async_SingleEvent", func(b *testing.B) {
		tmpfile, _ := os.CreateTemp("", "audit-latency-async-*.log") //nolint:errcheck // benchmark
		tmpfileName := tmpfile.Name()
		_ = tmpfile.Close()                           //nolint:errcheck // benchmark
		defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark

		logger, _ := NewAsyncAuditLogger(tmpfileName) //nolint:errcheck // benchmark
		defer func() { _ = logger.Close() }()         //nolint:errcheck // benchmark

		event := AuditEvent{EventType: "test", UserID: "user-123", Action: "read"}

		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			start := time.Now()
			_ = logger.Log(event) //nolint:errcheck // benchmark
			_ = time.Since(start) // Measure latency (would be recorded in real profiling)
		}
	})
}

// BenchmarkAuditLogger_CallerLatency measures the actual time the caller spends blocked
// This is the key metric - async should have much lower caller latency
func BenchmarkAuditLogger_CallerLatency(b *testing.B) {
	b.Run("Sync_CallerBlocked", func(b *testing.B) {
		tmpfile, _ := os.CreateTemp("", "audit-caller-sync-*.log") //nolint:errcheck // benchmark
		tmpfileName := tmpfile.Name()
		_ = tmpfile.Close()                           //nolint:errcheck // benchmark
		defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark

		logger, _ := NewAuditLogger(tmpfileName) //nolint:errcheck // benchmark
		defer func() { _ = logger.Close() }()    //nolint:errcheck // benchmark

		event := AuditEvent{
			EventType:  "benchmark_event",
			UserID:     "user-123",
			Username:   "testuser",
			TenantID:   "tenant-456",
			Action:     "read",
			Resource:   "article",
			Method:     "GET",
			Path:       "/api/v1/articles/123",
			StatusCode: 200,
			Duration:   10.5,
			IPAddress:  "192.168.1.1",
			UserAgent:  "Mozilla/5.0",
		}

		var totalLatency time.Duration
		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			start := time.Now()
			_ = logger.Log(event) //nolint:errcheck // benchmark
			totalLatency += time.Since(start)
		}
		b.ReportMetric(float64(totalLatency.Nanoseconds())/float64(b.N), "ns/call")
	})

	b.Run("Async_CallerUnblocked", func(b *testing.B) {
		tmpfile, _ := os.CreateTemp("", "audit-caller-async-*.log") //nolint:errcheck // benchmark
		tmpfileName := tmpfile.Name()
		_ = tmpfile.Close()                           //nolint:errcheck // benchmark
		defer func() { _ = os.Remove(tmpfileName) }() //nolint:errcheck // benchmark

		logger, _ := NewAsyncAuditLogger(tmpfileName) //nolint:errcheck // benchmark
		defer func() { _ = logger.Close() }()         //nolint:errcheck // benchmark

		event := AuditEvent{
			EventType:  "benchmark_event",
			UserID:     "user-123",
			Username:   "testuser",
			TenantID:   "tenant-456",
			Action:     "read",
			Resource:   "article",
			Method:     "GET",
			Path:       "/api/v1/articles/123",
			StatusCode: 200,
			Duration:   10.5,
			IPAddress:  "192.168.1.1",
			UserAgent:  "Mozilla/5.0",
		}

		var totalLatency time.Duration
		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			start := time.Now()
			_ = logger.Log(event) //nolint:errcheck // benchmark
			totalLatency += time.Since(start)
		}
		b.ReportMetric(float64(totalLatency.Nanoseconds())/float64(b.N), "ns/call")
	})
}
