package http

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func setupSuspiciousActivityHandler(t *testing.T) (*SuspiciousActivityHandler, *persistence.MemoryAuditRepository) {
	t.Helper()
	repo := persistence.NewMemoryAuditRepository()
	uc := usecases.NewDetectSuspiciousActivityUseCase(repo)
	handler := NewSuspiciousActivityHandler(uc)
	return handler, repo
}

func TestSuspiciousActivity_NoAlerts(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// Seed a few normal events — below all thresholds
	seedAuditEvents(t, repo, []*entities.AuditEvent{
		{Timestamp: now.Add(-10 * time.Minute), EventType: "api_call", UserID: "user1", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		{Timestamp: now.Add(-5 * time.Minute), EventType: "api_call", UserID: "user2", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.2"},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(0) {
		t.Errorf("expected total=0, got %v", resp["total"])
	}

	alerts, ok := resp["alerts"].([]interface{})
	if !ok {
		t.Fatal("alerts not found in response")
	}
	if len(alerts) != 0 {
		t.Errorf("expected 0 alerts, got %d", len(alerts))
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["token-audit"]; !ok {
		t.Error("missing token-audit link")
	}
}

func TestSuspiciousActivity_FailedAuth(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// Generate 101 failed auth events for tenant-1 within the last hour
	events := make([]*entities.AuditEvent, 0, 101)
	for i := 0; i < 101; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 30 * time.Second),
			EventType:  "api_call",
			UserID:     "attacker",
			TenantID:   "tenant-1",
			Path:       "/api/v1/auth/login",
			StatusCode: 401,
			IPAddress:  "10.0.0.1",
		})
	}
	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(1) {
		t.Fatalf("expected total=1, got %v", resp["total"])
	}

	alerts := resp["alerts"].([]interface{})    //nolint:errcheck,forcetypeassert
	alert := alerts[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert

	if alert["type"] != "excessive_failed_auth" {
		t.Errorf("expected type=excessive_failed_auth, got %v", alert["type"])
	}
	if alert["tenant_id"] != "tenant-1" {
		t.Errorf("expected tenant_id=tenant-1, got %v", alert["tenant_id"])
	}
	if alert["severity"] != "high" {
		t.Errorf("expected severity=high, got %v", alert["severity"])
	}

	details := alert["details"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if details["failed_count"] != float64(101) {
		t.Errorf("expected failed_count=101, got %v", details["failed_count"])
	}
	if details["threshold"] != float64(100) {
		t.Errorf("expected threshold=100, got %v", details["threshold"])
	}
}

func TestSuspiciousActivity_ExcessiveAPIKeyCreation(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// Generate 11 API key creation events for tenant-2 within the last hour
	events := make([]*entities.AuditEvent, 0, 11)
	for i := 0; i < 11; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * time.Minute),
			EventType:  "api_key_create",
			UserID:     "admin-user",
			TenantID:   "tenant-2",
			Action:     "create",
			Resource:   "api_key",
			Path:       "/api/v1/keys",
			StatusCode: 200,
			IPAddress:  "10.0.0.5",
		})
	}
	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(1) {
		t.Fatalf("expected total=1, got %v", resp["total"])
	}

	alerts := resp["alerts"].([]interface{})    //nolint:errcheck,forcetypeassert
	alert := alerts[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert

	if alert["type"] != "excessive_api_key_creation" {
		t.Errorf("expected type=excessive_api_key_creation, got %v", alert["type"])
	}
	if alert["tenant_id"] != "tenant-2" {
		t.Errorf("expected tenant_id=tenant-2, got %v", alert["tenant_id"])
	}
	if alert["severity"] != "medium" {
		t.Errorf("expected severity=medium, got %v", alert["severity"])
	}

	details := alert["details"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if details["keys_created"] != float64(11) {
		t.Errorf("expected keys_created=11, got %v", details["keys_created"])
	}
}

func TestSuspiciousActivity_UniqueIPBurst(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// Generate 21 events from unique IPs for tenant-3 within the last 5 minutes
	events := make([]*entities.AuditEvent, 0, 21)
	for i := 0; i < 21; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 10 * time.Second),
			EventType:  "api_call",
			UserID:     "user-x",
			TenantID:   "tenant-3",
			Path:       "/api/v1/events",
			StatusCode: 200,
			IPAddress:  "10.0.0." + itoa(i+1),
		})
	}
	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(1) {
		t.Fatalf("expected total=1, got %v", resp["total"])
	}

	alerts := resp["alerts"].([]interface{})    //nolint:errcheck,forcetypeassert
	alert := alerts[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert

	if alert["type"] != "unique_ip_burst" {
		t.Errorf("expected type=unique_ip_burst, got %v", alert["type"])
	}
	if alert["tenant_id"] != "tenant-3" {
		t.Errorf("expected tenant_id=tenant-3, got %v", alert["tenant_id"])
	}
	if alert["severity"] != "high" {
		t.Errorf("expected severity=high, got %v", alert["severity"])
	}

	details := alert["details"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if details["unique_ip_count"] != float64(21) {
		t.Errorf("expected unique_ip_count=21, got %v", details["unique_ip_count"])
	}
}

func TestSuspiciousActivity_BelowThreshold_NoAlert(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// 100 failed auth (exactly at threshold, not above) — should NOT trigger
	events := make([]*entities.AuditEvent, 0, 100)
	for i := 0; i < 100; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 30 * time.Second),
			EventType:  "api_call",
			UserID:     "attacker",
			TenantID:   "tenant-1",
			Path:       "/api/v1/auth/login",
			StatusCode: 401,
			IPAddress:  "10.0.0.1",
		})
	}
	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(0) {
		t.Errorf("expected total=0 (at threshold, not above), got %v", resp["total"])
	}
}

func TestSuspiciousActivity_MultipleAlerts(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()

	// 101 failed auth for tenant-1
	events := make([]*entities.AuditEvent, 0, 133)
	for i := 0; i < 101; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 30 * time.Second),
			EventType:  "api_call",
			UserID:     "attacker",
			TenantID:   "tenant-1",
			Path:       "/api/v1/auth/login",
			StatusCode: 401,
			IPAddress:  "10.0.0.1",
		})
	}

	// 11 API key creations for tenant-2
	for i := 0; i < 11; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * time.Minute),
			EventType:  "api_key_create",
			UserID:     "admin",
			TenantID:   "tenant-2",
			Action:     "create",
			Resource:   "api_key",
			Path:       "/api/v1/keys",
			StatusCode: 200,
			IPAddress:  "10.0.0.5",
		})
	}

	// 21 unique IPs for tenant-3
	for i := 0; i < 21; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 10 * time.Second),
			EventType:  "api_call",
			UserID:     "user-x",
			TenantID:   "tenant-3",
			Path:       "/api/v1/events",
			StatusCode: 200,
			IPAddress:  "192.168.1." + itoa(i+1),
		})
	}

	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	total := resp["total"].(float64) //nolint:errcheck,forcetypeassert
	if total != 3 {
		t.Errorf("expected 3 alerts (one per rule), got %v", total)
	}

	alerts := resp["alerts"].([]interface{}) //nolint:errcheck,forcetypeassert
	typeMap := make(map[string]bool)
	for _, a := range alerts {
		alert := a.(map[string]interface{})    //nolint:errcheck,forcetypeassert
		typeMap[alert["type"].(string)] = true //nolint:errcheck,forcetypeassert
	}

	if !typeMap["excessive_failed_auth"] {
		t.Error("missing excessive_failed_auth alert")
	}
	if !typeMap["excessive_api_key_creation"] {
		t.Error("missing excessive_api_key_creation alert")
	}
	if !typeMap["unique_ip_burst"] {
		t.Error("missing unique_ip_burst alert")
	}
}

func TestSuspiciousActivity_AlertStructure(t *testing.T) {
	handler, repo := setupSuspiciousActivityHandler(t)

	now := time.Now()
	// Trigger one alert to verify full structure
	events := make([]*entities.AuditEvent, 0, 101)
	for i := 0; i < 101; i++ {
		events = append(events, &entities.AuditEvent{
			Timestamp:  now.Add(-time.Duration(i) * 30 * time.Second),
			EventType:  "api_call",
			UserID:     "attacker",
			TenantID:   "tenant-1",
			Path:       "/api/v1/auth/login",
			StatusCode: 401,
			IPAddress:  "10.0.0.1",
		})
	}
	seedAuditEvents(t, repo, events)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/suspicious-activity", http.NoBody)

	handler.Detect(c)

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	alerts := resp["alerts"].([]interface{})    //nolint:errcheck,forcetypeassert
	alert := alerts[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert

	// Verify all required fields exist
	requiredFields := []string{"type", "tenant_id", "description", "severity", "timestamp", "details"}
	for _, field := range requiredFields {
		if _, ok := alert[field]; !ok {
			t.Errorf("missing required field: %s", field)
		}
	}
}

// itoa converts an int to a string without importing strconv to keep test simple.
func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	result := ""
	for n > 0 {
		result = string(rune('0'+n%10)) + result
		n /= 10
	}
	return result
}
