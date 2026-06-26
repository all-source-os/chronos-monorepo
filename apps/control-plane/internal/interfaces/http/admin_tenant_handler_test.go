package http

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// adminTenantMockCore implements the two CoreClient methods the admin tenant
// count-enrichment touches: GetTenantStats (real event totals) and GetConfig
// (the team-members list). The embedded interface satisfies the rest and panics
// if an un-mocked method is called, catching accidental new dependencies.
type adminTenantMockCore struct {
	clients.CoreClient

	statsByTenant   map[string]int64 // tenant id → event total
	membersByTenant map[string][]any // tenant id → member list (any element shape)
	statsErr        map[string]bool  // tenant id → force GetTenantStats error
	configErr       map[string]bool  // tenant id → force GetConfig error
}

func newAdminTenantMockCore() *adminTenantMockCore {
	return &adminTenantMockCore{
		statsByTenant:   map[string]int64{},
		membersByTenant: map[string][]any{},
		statsErr:        map[string]bool{},
		configErr:       map[string]bool{},
	}
}

func (m *adminTenantMockCore) GetTenantStats(_ context.Context, tenantID string) (*clients.TenantStatsResponse, error) {
	if m.statsErr[tenantID] {
		return nil, fmt.Errorf("core stats unavailable for %s", tenantID)
	}
	return &clients.TenantStatsResponse{TenantID: tenantID, EventCount: m.statsByTenant[tenantID]}, nil
}

func (m *adminTenantMockCore) GetConfig(_ context.Context, key string) (*clients.ConfigEntryResponse, error) {
	// key is "team:<tenantID>:members"; recover the tenant id.
	tenantID := strings.TrimSuffix(strings.TrimPrefix(key, "team:"), ":members")
	if m.configErr[tenantID] {
		return nil, fmt.Errorf("core config unavailable for %s", tenantID)
	}
	members, ok := m.membersByTenant[tenantID]
	if !ok {
		return nil, nil // no member list stored → clean 0
	}
	return &clients.ConfigEntryResponse{Key: key, Value: members}, nil
}

func setupAdminTenantHandler(t *testing.T) (*AdminTenantHandler, *persistence.MemoryTenantRepository) {
	t.Helper()
	handler, repo, _ := setupAdminTenantHandlerWithCore(t, nil)
	return handler, repo
}

// setupAdminTenantHandlerWithCore wires the handler with a (possibly nil) Core
// client so tests can exercise both the live-stats path and the metadata
// fallback. Passing nil reproduces production-with-Core-absent (metadata mirror).
func setupAdminTenantHandlerWithCore(t *testing.T, core clients.CoreClient) (*AdminTenantHandler, *persistence.MemoryTenantRepository, clients.CoreClient) {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()
	listTenantsUC := usecases.NewListTenantsUseCase(repo, core)
	getDetailUC := usecases.NewGetAdminTenantDetailUseCase(repo, core)
	getUsageUC := usecases.NewGetTenantUsageUseCase(repo, core)
	updateQuotasUC := usecases.NewUpdateTenantQuotasUseCase(repo, auditRepo)
	suspendTenantUC := usecases.NewSuspendTenantUseCase(repo, auditRepo)
	activateTenantUC := usecases.NewActivateTenantUseCase(repo, auditRepo)
	bulkTenantUC := usecases.NewBulkTenantUseCase(repo, auditRepo)
	// Read-only analysis use case wired with the same Core client; fleet/billing
	// deps nil here (the handler-level test only asserts the route is reachable +
	// read-only — the analysis logic has its own unit tests).
	analyzeUC := usecases.NewAnalyzeTenantsUseCase(repo, core, nil, nil)
	handler := NewAdminTenantHandler(listTenantsUC, getDetailUC, getUsageUC, updateQuotasUC, suspendTenantUC, activateTenantUC, bulkTenantUC, analyzeUC)
	return handler, repo, core
}

func seedTenants(t *testing.T, repo *persistence.MemoryTenantRepository, tenants []*entities.Tenant) {
	t.Helper()
	for _, tenant := range tenants {
		if err := repo.Save(tenant); err != nil {
			t.Fatalf("failed to seed tenant: %v", err)
		}
	}
}

func TestAdminListTenants_Basic(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Alpha Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{"tier": "pro"},
			"quotas":       map[string]interface{}{"events_used": float64(1500)},
			"member_count": float64(5),
		}},
		{ID: "t-2", Name: "Beta Inc", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{"tier": "free"},
		}},
		{ID: "t-3", Name: "Gamma LLC", Status: entities.TenantStatusSuspended, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants", http.NoBody)

	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	// Verify pagination metadata
	if resp["total"] != float64(3) {
		t.Errorf("expected total=3, got %v", resp["total"])
	}
	if resp["page"] != float64(1) {
		t.Errorf("expected page=1, got %v", resp["page"])
	}
	if resp["per_page"] != float64(20) {
		t.Errorf("expected per_page=20, got %v", resp["per_page"])
	}
	if resp["total_pages"] != float64(1) {
		t.Errorf("expected total_pages=1, got %v", resp["total_pages"])
	}

	// Verify tenants are present
	tenants, ok := resp["tenants"].([]interface{})
	if !ok {
		t.Fatalf("tenants not found in response")
	}
	if len(tenants) != 3 {
		t.Errorf("expected 3 tenants, got %d", len(tenants))
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
}

func TestAdminListTenants_SearchFilter(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Alpha Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-2", Name: "Beta Inc", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-3", Name: "Alpha Beta", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants?search=alpha", http.NoBody)

	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(2) {
		t.Errorf("expected total=2 for search=alpha, got %v", resp["total"])
	}
}

func TestAdminListTenants_PlanFilter(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Pro Tenant", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{"tier": "pro"},
		}},
		{ID: "t-2", Name: "Free Tenant", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-3", Name: "Team Tenant", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{"tier": "team"},
		}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants?plan=pro", http.NoBody)

	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(1) {
		t.Errorf("expected total=1 for plan=pro, got %v", resp["total"])
	}

	tenants := resp["tenants"].([]interface{})    //nolint:errcheck,forcetypeassert
	tenant := tenants[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if tenant["plan"] != "pro" {
		t.Errorf("expected plan=pro, got %v", tenant["plan"])
	}
}

func TestAdminListTenants_StatusFilter(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Active", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-2", Name: "Suspended", Status: entities.TenantStatusSuspended, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-3", Name: "Active2", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants?status=suspended", http.NoBody)

	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total"] != float64(1) {
		t.Errorf("expected total=1 for status=suspended, got %v", resp["total"])
	}
}

func TestAdminListTenants_Pagination(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	for i := 0; i < 5; i++ {
		seedTenants(t, repo, []*entities.Tenant{
			{ID: fmt.Sprintf("t-%d", i), Name: fmt.Sprintf("Tenant %d", i), Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		})
	}

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants?page=2&per_page=2", http.NoBody)

	handler.ListTenants(c)

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

	tenants := resp["tenants"].([]interface{}) //nolint:errcheck,forcetypeassert
	if len(tenants) != 2 {
		t.Errorf("expected 2 tenants on page 2, got %d", len(tenants))
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

func TestAdminListTenants_EnrichedFields(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-enriched", Name: "Enriched Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{"tier": "team"},
			"quotas":       map[string]interface{}{"events_used": float64(42000)},
			"member_count": float64(12),
		}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants", http.NoBody)

	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	tenants := resp["tenants"].([]interface{})    //nolint:errcheck,forcetypeassert
	tenant := tenants[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert

	if tenant["id"] != "t-enriched" {
		t.Errorf("expected id=t-enriched, got %v", tenant["id"])
	}
	if tenant["plan"] != "team" {
		t.Errorf("expected plan=team, got %v", tenant["plan"])
	}
	if tenant["event_count"] != float64(42000) {
		t.Errorf("expected event_count=42000, got %v", tenant["event_count"])
	}
	if tenant["member_count"] != float64(12) {
		t.Errorf("expected member_count=12, got %v", tenant["member_count"])
	}
	if tenant["status"] != "active" {
		t.Errorf("expected status=active, got %v", tenant["status"])
	}
}

func TestAdminGetDetail_ExistingTenant(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query:3902")

	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-detail", Name: "Detail Corp", Description: "A detailed tenant", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"subscription": map[string]interface{}{
				"tier":        "pro",
				"provider":    "comp",
				"external_id": "sub_123",
				"status":      "active",
				"plan_name":   "Pro Monthly",
			},
			"quotas": map[string]interface{}{
				"event_limit":      float64(100000),
				"query_limit":      float64(50000),
				"storage_limit_mb": float64(1024),
			},
			"member_count": float64(8),
		}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id", handler.GetDetail)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/t-detail", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	// Verify basic fields
	if resp["id"] != "t-detail" {
		t.Errorf("expected id=t-detail, got %v", resp["id"])
	}
	if resp["name"] != "Detail Corp" {
		t.Errorf("expected name=Detail Corp, got %v", resp["name"])
	}
	if resp["description"] != "A detailed tenant" {
		t.Errorf("expected description, got %v", resp["description"])
	}
	if resp["status"] != "active" {
		t.Errorf("expected status=active, got %v", resp["status"])
	}
	if resp["member_count"] != float64(8) {
		t.Errorf("expected member_count=8, got %v", resp["member_count"])
	}

	// Verify plan
	plan, ok := resp["plan"].(map[string]interface{})
	if !ok {
		t.Fatal("plan not found in response")
	}
	if plan["tier"] != "pro" {
		t.Errorf("expected plan tier=pro, got %v", plan["tier"])
	}

	// Verify quotas
	quotas, ok := resp["quotas"].(map[string]interface{})
	if !ok {
		t.Fatal("quotas not found in response")
	}
	if quotas["event_limit"] != float64(100000) {
		t.Errorf("expected event_limit=100000, got %v", quotas["event_limit"])
	}

	// Verify subscription
	sub, ok := resp["subscription"].(map[string]interface{})
	if !ok {
		t.Fatal("subscription not found in response")
	}
	if sub["provider"] != "comp" {
		t.Errorf("expected provider=comp, got %v", sub["provider"])
	}
	if sub["external_id"] != "sub_123" {
		t.Errorf("expected external_id=sub_123, got %v", sub["external_id"])
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	expectedLinkNames := []string{"self", "usage", "billing", "audit", "suspend", "events"}
	for _, name := range expectedLinkNames {
		if _, exists := links[name]; !exists {
			t.Errorf("missing HAL link: %s", name)
		}
	}

	// Verify self link href
	selfLink, ok := links["self"].(map[string]interface{})
	if !ok {
		t.Fatal("self link not found")
	}
	if selfLink["href"] != "/api/v1/admin/tenants/t-detail" {
		t.Errorf("expected self href /api/v1/admin/tenants/t-detail, got %v", selfLink["href"])
	}
}

func TestAdminGetDetail_NotFound(t *testing.T) {
	handler, _ := setupAdminTenantHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id", handler.GetDetail)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/nonexistent", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}
	if _, ok := resp["error"]; !ok {
		t.Error("expected error field in 404 response")
	}
}

// ── Real-count enrichment (Gap 3): list + detail source counts from Core ──────

func TestAdminListTenants_RealCountsFromCore(t *testing.T) {
	core := newAdminTenantMockCore()
	// Tenant with real usage: 7777 events in Core, 3 members in the team config.
	core.statsByTenant["t-live"] = 7777
	core.membersByTenant["t-live"] = []any{
		map[string]any{"email": "a@x.io"},
		map[string]any{"email": "b@x.io"},
		map[string]any{"email": "c@x.io"},
	}
	// Zero-usage tenant: no stats, no member list → must render a clean 0.
	core.statsByTenant["t-zero"] = 0

	handler, repo, _ := setupAdminTenantHandlerWithCore(t, core)
	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		// Stale metadata mirror deliberately DISAGREES with live Core to prove
		// the live value wins (events_used=1, member_count=1 in metadata).
		{ID: "t-live", Name: "Live Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"quotas":       map[string]interface{}{"events_used": float64(1)},
			"member_count": float64(1),
		}},
		{ID: "t-zero", Name: "Zero Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants", http.NoBody)
	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}
	byID := map[string]map[string]interface{}{}
	for _, raw := range resp["tenants"].([]interface{}) { //nolint:forcetypeassert
		tn := raw.(map[string]interface{}) //nolint:forcetypeassert
		byID[tn["id"].(string)] = tn       //nolint:forcetypeassert
	}

	live := byID["t-live"]
	if live["event_count"] != float64(7777) {
		t.Errorf("t-live event_count: want 7777 from Core stats, got %v", live["event_count"])
	}
	if live["member_count"] != float64(3) {
		t.Errorf("t-live member_count: want 3 from Core members config, got %v", live["member_count"])
	}

	zero := byID["t-zero"]
	if zero["event_count"] != float64(0) {
		t.Errorf("t-zero event_count: want clean 0, got %v", zero["event_count"])
	}
	if zero["member_count"] != float64(0) {
		t.Errorf("t-zero member_count: want clean 0, got %v", zero["member_count"])
	}
}

func TestAdminListTenants_FallsBackToMetadataOnCoreError(t *testing.T) {
	core := newAdminTenantMockCore()
	core.statsErr["t-x"] = true  // Core stats down → fall back to metadata mirror
	core.configErr["t-x"] = true // Core config down → fall back to metadata mirror

	handler, repo, _ := setupAdminTenantHandlerWithCore(t, core)
	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-x", Name: "Fallback Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			"quotas":       map[string]interface{}{"events_used": float64(500)},
			"member_count": float64(4),
		}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants", http.NoBody)
	handler.ListTenants(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}
	tn := resp["tenants"].([]interface{})[0].(map[string]interface{}) //nolint:forcetypeassert
	if tn["event_count"] != float64(500) {
		t.Errorf("event_count fallback: want 500 from metadata, got %v", tn["event_count"])
	}
	if tn["member_count"] != float64(4) {
		t.Errorf("member_count fallback: want 4 from metadata, got %v", tn["member_count"])
	}
}

func TestAdminGetDetail_RealCountsFromCore(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query:3902")

	core := newAdminTenantMockCore()
	core.statsByTenant["t-detail-live"] = 31337
	core.membersByTenant["t-detail-live"] = []any{
		map[string]any{"email": "x@y.io"},
		map[string]any{"email": "z@y.io"},
	}
	handler, repo, _ := setupAdminTenantHandlerWithCore(t, core)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-detail-live", Name: "Detail Live", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{
			// stale metadata disagrees; live Core must win
			"quotas":       map[string]interface{}{"events_used": float64(2)},
			"member_count": float64(99),
		}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id", handler.GetDetail)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/t-detail-live", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}
	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}
	if resp["event_count"] != float64(31337) {
		t.Errorf("detail event_count: want 31337 from Core stats, got %v", resp["event_count"])
	}
	if resp["member_count"] != float64(2) {
		t.Errorf("detail member_count: want 2 from Core members config, got %v", resp["member_count"])
	}
}

func TestAdminGetDetail_ZeroUsageCleanZero(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query:3902")

	core := newAdminTenantMockCore() // no stats, no members for this tenant
	handler, repo, _ := setupAdminTenantHandlerWithCore(t, core)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-detail-zero", Name: "Detail Zero", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id", handler.GetDetail)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/t-detail-zero", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}
	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}
	if resp["event_count"] != float64(0) {
		t.Errorf("zero-usage detail event_count: want 0, got %v", resp["event_count"])
	}
	if resp["member_count"] != float64(0) {
		t.Errorf("zero-usage detail member_count: want 0, got %v", resp["member_count"])
	}
}

func TestAdminGetUsage_ExistingTenant(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-usage", Name: "Usage Corp", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id/usage", handler.GetUsage)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/t-usage/usage", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	// Verify tenant_id
	if resp["tenant_id"] != "t-usage" {
		t.Errorf("expected tenant_id=t-usage, got %v", resp["tenant_id"])
	}

	// Verify usage fields are present (zeros without CoreClient)
	if resp["event_count"] != float64(0) {
		t.Errorf("expected event_count=0 without core client, got %v", resp["event_count"])
	}
	if resp["storage_bytes"] != float64(0) {
		t.Errorf("expected storage_bytes=0 without core client, got %v", resp["storage_bytes"])
	}

	// Verify daily_usage is a 7-day array
	daily, ok := resp["daily_usage"].([]interface{})
	if !ok {
		t.Fatal("daily_usage not found in response")
	}
	if len(daily) != 7 {
		t.Errorf("expected 7 days in daily_usage, got %d", len(daily))
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["tenant"]; !ok {
		t.Error("missing tenant link")
	}

	// Verify self link href
	selfLink := links["self"].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if selfLink["href"] != "/api/v1/admin/tenants/t-usage/usage" {
		t.Errorf("expected self href /api/v1/admin/tenants/t-usage/usage, got %v", selfLink["href"])
	}
}

func TestAdminGetUsage_NotFound(t *testing.T) {
	handler, _ := setupAdminTenantHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, r := gin.CreateTestContext(w)
	r.GET("/api/v1/admin/tenants/:id/usage", handler.GetUsage)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/tenants/nonexistent/usage", http.NoBody)
	r.ServeHTTP(w, c.Request)

	if w.Code != http.StatusNotFound {
		t.Fatalf("expected 404, got %d (body: %s)", w.Code, w.Body.String())
	}
}

func TestAdminTenantDetailLinks(t *testing.T) {
	t.Setenv("DATA_PLANE_URL", "http://query:3902")

	links := adminTenantDetailLinks("t-100")

	expectedLinkNames := []string{"self", "usage", "billing", "audit", "suspend", "events"}
	if len(links) != len(expectedLinkNames) {
		t.Fatalf("expected %d links, got %d", len(expectedLinkNames), len(links))
	}
	for _, name := range expectedLinkNames {
		if _, ok := links[name]; !ok {
			t.Errorf("missing link: %s", name)
		}
	}
	if links["self"].Href != "/api/v1/admin/tenants/t-100" {
		t.Errorf("unexpected self href: %s", links["self"].Href)
	}
	if links["usage"].Href != "/api/v1/admin/tenants/t-100/usage" {
		t.Errorf("unexpected usage href: %s", links["usage"].Href)
	}
	if !links["events"].Templated {
		t.Error("expected events link to be templated")
	}
}

func TestAdminTenantUsageLinks(t *testing.T) {
	links := adminTenantUsageLinks("t-200")

	if len(links) != 2 {
		t.Fatalf("expected 2 links, got %d", len(links))
	}
	if links["self"].Href != "/api/v1/admin/tenants/t-200/usage" {
		t.Errorf("unexpected self href: %s", links["self"].Href)
	}
	if links["tenant"].Href != "/api/v1/admin/tenants/t-200" {
		t.Errorf("unexpected tenant href: %s", links["tenant"].Href)
	}
}

func TestAdminTenantListLinks(t *testing.T) {
	// First page of multi-page results
	links := adminTenantListLinks(1, 10, 3)
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 1 of 3")
	}
	if _, ok := links["prev"]; ok {
		t.Error("unexpected prev link on page 1")
	}

	// Middle page
	links = adminTenantListLinks(2, 10, 3)
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on page 2")
	}
	if _, ok := links["next"]; !ok {
		t.Error("expected next link on page 2")
	}

	// Last page
	links = adminTenantListLinks(3, 10, 3)
	if _, ok := links["prev"]; !ok {
		t.Error("expected prev link on last page")
	}
	if _, ok := links["next"]; ok {
		t.Error("unexpected next link on last page")
	}
}

func TestAdminBulkSuspend_MixedValidInvalid(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Alpha", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
		{ID: "t-2", Name: "Beta", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	body := `{"action":"suspend","tenant_ids":["t-1","nonexistent","t-2"]}`
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/tenants/bulk", strings.NewReader(body))
	c.Request.Header.Set("Content-Type", "application/json")
	c.Set("auth_role", entities.RoleAdmin)

	handler.BulkAction(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["action"] != "suspend" {
		t.Errorf("expected action=suspend, got %v", resp["action"])
	}
	if resp["total"] != float64(3) {
		t.Errorf("expected total=3, got %v", resp["total"])
	}
	if resp["succeeded"] != float64(2) {
		t.Errorf("expected succeeded=2, got %v", resp["succeeded"])
	}
	if resp["failed"] != float64(1) {
		t.Errorf("expected failed=1, got %v", resp["failed"])
	}

	results, ok := resp["results"].([]interface{})
	if !ok {
		t.Fatal("results not found in response")
	}
	if len(results) != 3 {
		t.Fatalf("expected 3 results, got %d", len(results))
	}

	// Verify per-tenant results
	r0 := results[0].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if r0["tenant_id"] != "t-1" || r0["success"] != true {
		t.Errorf("expected t-1 success, got %v", r0)
	}

	r1 := results[1].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if r1["tenant_id"] != "nonexistent" || r1["success"] != false {
		t.Errorf("expected nonexistent failure, got %v", r1)
	}
	if _, hasError := r1["error"]; !hasError {
		t.Error("expected error field for nonexistent tenant")
	}

	r2 := results[2].(map[string]interface{}) //nolint:errcheck,forcetypeassert
	if r2["tenant_id"] != "t-2" || r2["success"] != true {
		t.Errorf("expected t-2 success, got %v", r2)
	}

	// Verify tenants are actually suspended in the repo
	tenant1, _ := repo.FindByID("t-1") //nolint:errcheck
	if tenant1.Status != entities.TenantStatusSuspended {
		t.Errorf("expected t-1 to be suspended, got %v", tenant1.Status)
	}
	tenant2, _ := repo.FindByID("t-2") //nolint:errcheck
	if tenant2.Status != entities.TenantStatusSuspended {
		t.Errorf("expected t-2 to be suspended, got %v", tenant2.Status)
	}
}

func TestAdminBulkAction_ValidationErrors(t *testing.T) {
	handler, _ := setupAdminTenantHandler(t)

	tests := []struct {
		name       string
		body       string
		wantStatus int
	}{
		{
			name:       "empty tenant_ids",
			body:       `{"action":"suspend","tenant_ids":[]}`,
			wantStatus: http.StatusBadRequest,
		},
		{
			name:       "invalid action",
			body:       `{"action":"delete_all","tenant_ids":["t-1"]}`,
			wantStatus: http.StatusBadRequest,
		},
		{
			name:       "invalid JSON",
			body:       `{invalid`,
			wantStatus: http.StatusBadRequest,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gin.SetMode(gin.TestMode)
			w := httptest.NewRecorder()
			c, _ := gin.CreateTestContext(w)

			c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/tenants/bulk", strings.NewReader(tt.body))
			c.Request.Header.Set("Content-Type", "application/json")
			c.Set("auth_role", entities.RoleAdmin)

			handler.BulkAction(c)

			if w.Code != tt.wantStatus {
				t.Errorf("expected %d, got %d (body: %s)", tt.wantStatus, w.Code, w.Body.String())
			}
		})
	}
}

func TestAdminBulkAction_ForbiddenWithoutAdmin(t *testing.T) {
	handler, _ := setupAdminTenantHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	body := `{"action":"suspend","tenant_ids":["t-1"]}`
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/tenants/bulk", strings.NewReader(body))
	c.Request.Header.Set("Content-Type", "application/json")
	c.Set("auth_role", entities.RoleDeveloper)

	handler.BulkAction(c)

	if w.Code != http.StatusForbidden {
		t.Errorf("expected 403, got %d (body: %s)", w.Code, w.Body.String())
	}
}

func TestAdminBulkArchive(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Alpha", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	body := `{"action":"archive","tenant_ids":["t-1"]}`
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/tenants/bulk", strings.NewReader(body))
	c.Request.Header.Set("Content-Type", "application/json")
	c.Set("auth_role", entities.RoleAdmin)

	handler.BulkAction(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["action"] != "archive" {
		t.Errorf("expected action=archive, got %v", resp["action"])
	}
	if resp["succeeded"] != float64(1) {
		t.Errorf("expected succeeded=1, got %v", resp["succeeded"])
	}

	// Verify tenant is archived
	tenant, _ := repo.FindByID("t-1") //nolint:errcheck
	if tenant.Status != entities.TenantStatusArchived {
		t.Errorf("expected t-1 to be archived, got %v", tenant.Status)
	}
}

func TestAdminBulkExport(t *testing.T) {
	handler, repo := setupAdminTenantHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{ID: "t-1", Name: "Alpha", Status: entities.TenantStatusActive, CreatedAt: now, UpdatedAt: now, Metadata: map[string]interface{}{}},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	body := `{"action":"export","tenant_ids":["t-1"]}`
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/tenants/bulk", strings.NewReader(body))
	c.Request.Header.Set("Content-Type", "application/json")
	c.Set("auth_role", entities.RoleAdmin)

	handler.BulkAction(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["action"] != "export" {
		t.Errorf("expected action=export, got %v", resp["action"])
	}
	if resp["succeeded"] != float64(1) {
		t.Errorf("expected succeeded=1, got %v", resp["succeeded"])
	}

	// Export returns an operation_id for async tracking
	opID, ok := resp["operation_id"].(string)
	if !ok || opID == "" {
		t.Error("expected non-empty operation_id for export action")
	}
}
