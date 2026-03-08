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

func setupTokenAuditHandler(t *testing.T) (*TokenAuditHandler, *persistence.MemoryAuditRepository) {
	t.Helper()
	repo := persistence.NewMemoryAuditRepository()
	uc := usecases.NewQueryTokenAuditUseCase(repo)
	handler := NewTokenAuditHandler(uc)
	return handler, repo
}

func seedAuditEvents(t *testing.T, repo *persistence.MemoryAuditRepository, events []*entities.AuditEvent) {
	t.Helper()
	for _, e := range events {
		if err := repo.Log(e); err != nil {
			t.Fatalf("failed to seed audit event: %v", err)
		}
	}
}

func TestTokenAuditQuery_Basic(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	seedAuditEvents(t, repo, []*entities.AuditEvent{
		{Timestamp: now.Add(-1 * time.Hour), EventType: "api_call", UserID: "apikey_abc12345xyz", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		{Timestamp: now.Add(-30 * time.Minute), EventType: "api_call", UserID: "apikey_def67890uvw", TenantID: "tenant-1", Path: "/api/v1/events/query", StatusCode: 200, IPAddress: "10.0.0.2"},
		{Timestamp: now.Add(-15 * time.Minute), EventType: "api_call", UserID: "apikey_ghi11111aaa", TenantID: "tenant-2", Path: "/api/v1/events", StatusCode: 500, IPAddress: "10.0.0.3"},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	from := now.Add(-2 * time.Hour).Format(time.RFC3339)
	to := now.Add(1 * time.Hour).Format(time.RFC3339)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?from="+from+"&to="+to, http.NoBody)

	handler.Query(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(3) {
		t.Errorf("expected total=3, got %v", resp["total"])
	}
	if resp["page"] != float64(1) {
		t.Errorf("expected page=1, got %v", resp["page"])
	}

	entries, entriesOK := resp["entries"].([]interface{})
	if !entriesOK {
		t.Fatal("entries not found in response")
	}
	if len(entries) != 3 {
		t.Errorf("expected 3 entries, got %d", len(entries))
	}

	// Verify entry structure
	entry := entries[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if _, exists := entry["tenant_id"]; !exists {
		t.Error("missing tenant_id in entry")
	}
	if _, exists := entry["key_prefix"]; !exists {
		t.Error("missing key_prefix in entry")
	}
	if _, exists := entry["endpoint"]; !exists {
		t.Error("missing endpoint in entry")
	}
	if _, exists := entry["timestamp"]; !exists {
		t.Error("missing timestamp in entry")
	}
	if _, exists := entry["response_status"]; !exists {
		t.Error("missing response_status in entry")
	}
	if _, exists := entry["ip_address"]; !exists {
		t.Error("missing ip_address in entry")
	}

	// Verify HAL links
	links, linksOK := resp["_links"].(map[string]interface{})
	if !linksOK {
		t.Fatal("_links not found in response")
	}
	if _, exists := links["self"]; !exists {
		t.Error("missing self link")
	}
	if _, exists := links["summary"]; !exists {
		t.Error("missing summary link")
	}
}

func TestTokenAuditQuery_KeyPrefixTruncation(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	seedAuditEvents(t, repo, []*entities.AuditEvent{
		{Timestamp: now.Add(-1 * time.Hour), EventType: "api_call", UserID: "apikey_abc12345xyz_longkey", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		{Timestamp: now.Add(-30 * time.Minute), EventType: "api_call", UserID: "short", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.2"},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	from := now.Add(-2 * time.Hour).Format(time.RFC3339)
	to := now.Add(1 * time.Hour).Format(time.RFC3339)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?from="+from+"&to="+to, http.NoBody)

	handler.Query(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	entries := resp["entries"].([]interface{}) //nolint:errcheck,forcetypeassert

	// First entry: key should be truncated to 8 chars
	entry0 := entries[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if entry0["key_prefix"] != "apikey_a" {
		t.Errorf("expected key_prefix='apikey_a' (8 chars), got %v", entry0["key_prefix"])
	}

	// Second entry: key shorter than 8 chars, should be used as-is
	entry1 := entries[1].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if entry1["key_prefix"] != "short" {
		t.Errorf("expected key_prefix='short', got %v", entry1["key_prefix"])
	}
}

func TestTokenAuditQuery_TenantFilter(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	seedAuditEvents(t, repo, []*entities.AuditEvent{
		{Timestamp: now.Add(-1 * time.Hour), EventType: "api_call", UserID: "key1", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		{Timestamp: now.Add(-30 * time.Minute), EventType: "api_call", UserID: "key2", TenantID: "tenant-2", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.2"},
		{Timestamp: now.Add(-15 * time.Minute), EventType: "api_call", UserID: "key3", TenantID: "tenant-1", Path: "/api/v1/events/query", StatusCode: 200, IPAddress: "10.0.0.3"},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?tenant_id=tenant-1", http.NoBody)

	handler.Query(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(2) {
		t.Errorf("expected total=2 for tenant-1, got %v", resp["total"])
	}

	entries := resp["entries"].([]interface{}) //nolint:errcheck,forcetypeassert
	for _, e := range entries {
		entry := e.(map[string]interface{}) //nolint:errcheck,forcetypeassert
		if entry["tenant_id"] != "tenant-1" {
			t.Errorf("expected tenant_id=tenant-1, got %v", entry["tenant_id"])
		}
	}
}

func TestTokenAuditQuery_Pagination(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	// Create 5 events for the same tenant
	for i := 0; i < 5; i++ {
		seedAuditEvents(t, repo, []*entities.AuditEvent{
			{Timestamp: now.Add(-time.Duration(i) * time.Hour), EventType: "api_call", UserID: "key", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		})
	}

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?tenant_id=tenant-1&page=2&per_page=2", http.NoBody)

	handler.Query(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(5) {
		t.Errorf("expected total=5, got %v", resp["total"])
	}
	if resp["page"] != float64(2) {
		t.Errorf("expected page=2, got %v", resp["page"])
	}
	if resp["per_page"] != float64(2) {
		t.Errorf("expected per_page=2, got %v", resp["per_page"])
	}
	if resp["total_pages"] != float64(3) {
		t.Errorf("expected total_pages=3, got %v", resp["total_pages"])
	}

	entries := resp["entries"].([]interface{}) //nolint:errcheck,forcetypeassert
	if len(entries) != 2 {
		t.Errorf("expected 2 entries on page 2, got %d", len(entries))
	}

	// Verify HAL pagination links
	links := resp["_links"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on page 2")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 2")
	}
	if _, ok := links["first"]; !ok {
		t.Error("expected first link on page 2")
	}
	if _, ok := links["last"]; !ok {
		t.Error("expected last link on page 2")
	}
}

func TestTokenAuditQuery_PaginationFirstPage(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	for i := 0; i < 5; i++ {
		seedAuditEvents(t, repo, []*entities.AuditEvent{
			{Timestamp: now.Add(-time.Duration(i) * time.Hour), EventType: "api_call", UserID: "key", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		})
	}

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?tenant_id=tenant-1&page=1&per_page=2", http.NoBody)

	handler.Query(c)

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links := resp["_links"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if _, ok := links["prev"]; ok {
		t.Error("unexpected prev link on page 1")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 1")
	}
}

func TestTokenAuditQuery_PaginationLastPage(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	for i := 0; i < 4; i++ {
		seedAuditEvents(t, repo, []*entities.AuditEvent{
			{Timestamp: now.Add(-time.Duration(i) * time.Hour), EventType: "api_call", UserID: "key", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		})
	}

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?tenant_id=tenant-1&page=2&per_page=2", http.NoBody)

	handler.Query(c)

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links := resp["_links"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on last page")
	}
	if _, ok := links["next"]; ok {
		t.Error("unexpected next link on last page")
	}
}

func TestTokenAuditQuery_EmptyResults(t *testing.T) {
	handler, _ := setupTokenAuditHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit?tenant_id=nonexistent", http.NoBody)

	handler.Query(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(0) {
		t.Errorf("expected total=0, got %v", resp["total"])
	}

	entries, _ := resp["entries"].([]interface{}) //nolint:errcheck
	if len(entries) != 0 {
		t.Errorf("expected 0 entries, got %d", len(entries))
	}
}

func TestTokenAuditSummary_Basic(t *testing.T) {
	handler, repo := setupTokenAuditHandler(t)

	now := time.Now()
	seedAuditEvents(t, repo, []*entities.AuditEvent{
		{Timestamp: now.Add(-1 * time.Hour), EventType: "api_call", UserID: "key1", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.1"},
		{Timestamp: now.Add(-30 * time.Minute), EventType: "api_call", UserID: "key2", TenantID: "tenant-1", Path: "/api/v1/events", StatusCode: 500, IPAddress: "10.0.0.2"},
		{Timestamp: now.Add(-15 * time.Minute), EventType: "api_call", UserID: "key3", TenantID: "tenant-2", Path: "/api/v1/events", StatusCode: 200, IPAddress: "10.0.0.3"},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	from := now.Add(-2 * time.Hour).Format(time.RFC3339)
	to := now.Add(1 * time.Hour).Format(time.RFC3339)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit/summary?from="+from+"&to="+to, http.NoBody)

	handler.Summary(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(2) {
		t.Errorf("expected total=2 tenants, got %v", resp["total"])
	}

	tenants, tenantsOK := resp["tenants"].([]interface{})
	if !tenantsOK {
		t.Fatal("tenants not found in response")
	}
	if len(tenants) != 2 {
		t.Errorf("expected 2 tenant summaries, got %d", len(tenants))
	}

	// Build a map of tenant summaries for easier assertion
	tenantMap := make(map[string]map[string]interface{})
	for _, te := range tenants {
		tenant := te.(map[string]interface{})            //nolint:errcheck,forcetypeassert
		tenantMap[tenant["tenant_id"].(string)] = tenant //nolint:errcheck,forcetypeassert
	}

	// tenant-1: 2 calls, 1 error
	if t1, found := tenantMap["tenant-1"]; found {
		if t1["total_calls"] != float64(2) {
			t.Errorf("expected tenant-1 total_calls=2, got %v", t1["total_calls"])
		}
		if t1["error_count"] != float64(1) {
			t.Errorf("expected tenant-1 error_count=1, got %v", t1["error_count"])
		}
	} else {
		t.Error("tenant-1 not found in summary")
	}

	// tenant-2: 1 call, 0 errors
	if t2, found := tenantMap["tenant-2"]; found {
		if t2["total_calls"] != float64(1) {
			t.Errorf("expected tenant-2 total_calls=1, got %v", t2["total_calls"])
		}
		if t2["error_count"] != float64(0) {
			t.Errorf("expected tenant-2 error_count=0, got %v", t2["error_count"])
		}
	} else {
		t.Error("tenant-2 not found in summary")
	}

	// Verify HAL links
	links, linksOK := resp["_links"].(map[string]interface{})
	if !linksOK {
		t.Fatal("_links not found in response")
	}
	if _, exists := links["self"]; !exists {
		t.Error("missing self link")
	}
	if _, exists := links["list"]; !exists {
		t.Error("missing list link")
	}
}

func TestTokenAuditSummary_Empty(t *testing.T) {
	handler, _ := setupTokenAuditHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	from := time.Now().Add(-2 * time.Hour).Format(time.RFC3339)
	to := time.Now().Add(1 * time.Hour).Format(time.RFC3339)
	c.Request = httptest.NewRequest(http.MethodGet, "/api/v1/admin/security/token-audit/summary?from="+from+"&to="+to, http.NoBody)

	handler.Summary(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(0) {
		t.Errorf("expected total=0, got %v", resp["total"])
	}

	tenants := resp["tenants"].([]interface{}) //nolint:errcheck,forcetypeassert
	if len(tenants) != 0 {
		t.Errorf("expected 0 tenant summaries, got %d", len(tenants))
	}
}

func TestTokenAuditListLinks(t *testing.T) {
	// First page
	links := tokenAuditListLinks(1, 20, 3, "")
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 1 of 3")
	}
	if _, ok := links["prev"]; ok {
		t.Error("unexpected prev link on page 1")
	}
	if _, ok := links["summary"]; !ok {
		t.Error("missing summary link")
	}

	// Middle page
	links = tokenAuditListLinks(2, 20, 3, "")
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on page 2")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 2")
	}

	// Last page
	links = tokenAuditListLinks(3, 20, 3, "")
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on last page")
	}
	if _, ok := links["next"]; ok {
		t.Error("unexpected next link on last page")
	}

	// With tenant_id filter
	links = tokenAuditListLinks(1, 20, 1, "tenant-1")
	selfLink := links["self"]
	if selfLink.Href != "/api/v1/admin/security/token-audit?page=1&per_page=20&tenant_id=tenant-1" {
		t.Errorf("unexpected self href: %s", selfLink.Href)
	}
}
