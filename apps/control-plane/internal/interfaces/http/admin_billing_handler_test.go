package http

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases/billing"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

func setupAdminBillingHandler(t *testing.T) (*AdminBillingHandler, *persistence.MemoryTenantRepository) {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	auditRepo := persistence.NewMemoryAuditRepository()

	// Revenue and dunning use cases don't require LSClient
	revenueUC := billing.NewAdminRevenueUseCase(repo, auditRepo, nil)
	dunningUC := billing.NewAdminDunningUseCase(repo, auditRepo)

	// Invoices and refund use cases require LSClient — nil for these tests
	handler := NewAdminBillingHandler(nil, revenueUC, nil, dunningUC)
	return handler, repo
}

func setAdminAuth(c *gin.Context) {
	c.Set("admin_auth", &AdminAuthContext{
		UserID:   "admin-1",
		Username: "admin",
		TenantID: "system",
		Role:     entities.RoleAdmin,
	})
}

func TestAdminBillingGetRevenue_Basic(t *testing.T) {
	handler, repo := setupAdminBillingHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{
			ID: "t-1", Name: "Pro Corp", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		{
			ID: "t-2", Name: "Team Inc", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "team", Status: "active"},
			},
		},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/revenue?range=30d", http.NoBody)
	setAdminAuth(c)

	handler.GetRevenue(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	// MRR should be $29 + $99 = $128
	if resp["mrr"] != float64(128) {
		t.Errorf("expected mrr=128, got %v", resp["mrr"])
	}
	if resp["period"] != defaultRevenueRange {
		t.Errorf("expected period=30d, got %v", resp["period"])
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["invoices"]; !ok {
		t.Error("missing invoices link")
	}
}

func TestAdminBillingGetRevenue_InvalidRange(t *testing.T) {
	handler, _ := setupAdminBillingHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/revenue?range=invalid", http.NoBody)
	setAdminAuth(c)

	handler.GetRevenue(c)

	if w.Code != http.StatusBadRequest {
		t.Fatalf("expected 400, got %d", w.Code)
	}
}

func TestAdminBillingGetRevenue_DefaultRange(t *testing.T) {
	handler, _ := setupAdminBillingHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/revenue", http.NoBody)
	setAdminAuth(c)

	handler.GetRevenue(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["period"] != defaultRevenueRange {
		t.Errorf("expected default period=30d, got %v", resp["period"])
	}
}

func TestAdminBillingGetDunning_NoPastDue(t *testing.T) {
	handler, repo := setupAdminBillingHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{
			ID: "t-1", Name: "Active Corp", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/dunning", http.NoBody)
	setAdminAuth(c)

	handler.GetDunning(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total_count"] != float64(0) {
		t.Errorf("expected total_count=0, got %v", resp["total_count"])
	}
}

func TestAdminBillingGetDunning_WithPastDue(t *testing.T) {
	handler, repo := setupAdminBillingHandler(t)

	now := time.Now()
	seedTenants(t, repo, []*entities.Tenant{
		{
			ID: "t-1", Name: "Active Corp", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{Tier: "pro", Status: "active"},
			},
		},
		{
			ID: "t-2", Name: "Past Due Corp", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -3, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{
					Tier:           "team",
					Status:         "past_due",
					SubscriptionID: "sub_123",
					CustomerID:     "cust_456",
				},
			},
		},
		{
			ID: "t-3", Name: "Expired Corp", Status: entities.TenantStatusActive,
			CreatedAt: now.AddDate(0, -6, 0), UpdatedAt: now,
			Metadata: map[string]interface{}{
				"subscription": &entities.SubscriptionMetadata{
					Tier:           "pro",
					Status:         "expired",
					SubscriptionID: "sub_789",
				},
			},
		},
	})

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/dunning", http.NoBody)
	setAdminAuth(c)

	handler.GetDunning(c)

	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (body: %s)", w.Code, w.Body.String())
	}

	var resp map[string]interface{}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	if resp["total_count"] != float64(2) {
		t.Errorf("expected total_count=2, got %v", resp["total_count"])
	}

	items := resp["items"].([]interface{}) //nolint:errcheck,forcetypeassert
	if len(items) != 2 {
		t.Fatalf("expected 2 dunning items, got %d", len(items))
	}

	itemsByTenant := make(map[string]map[string]interface{}, len(items))
	for _, item := range items {
		itemMap := item.(map[string]interface{})  //nolint:errcheck,forcetypeassert
		tenantID := itemMap["tenant_id"].(string) //nolint:errcheck,forcetypeassert
		itemsByTenant[tenantID] = itemMap
	}

	item2, ok := itemsByTenant["t-2"]
	if !ok {
		t.Fatal("expected dunning item for tenant t-2")
	}
	if item2["status"] != "past_due" {
		t.Errorf("expected status=past_due, got %v", item2["status"])
	}
	if item2["retry_status"] != "pending_retry" {
		t.Errorf("expected retry_status=pending_retry, got %v", item2["retry_status"])
	}

	item3, ok := itemsByTenant["t-3"]
	if !ok {
		t.Fatal("expected dunning item for tenant t-3")
	}
	if item3["retry_status"] != "manual_review" {
		t.Errorf("expected retry_status=manual_review, got %v", item3["retry_status"])
	}

	// Verify HAL links
	links, ok := resp["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in response")
	}
	if _, ok := links["self"]; !ok {
		t.Error("missing self link")
	}
	if _, ok := links["invoices"]; !ok {
		t.Error("missing invoices link")
	}
	if _, ok := links["revenue"]; !ok {
		t.Error("missing revenue link")
	}
}

func TestAdminBillingListInvoices_NoLSClient(t *testing.T) {
	handler, _ := setupAdminBillingHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/admin/billing/invoices", http.NoBody)
	setAdminAuth(c)

	// listInvoicesUC is nil because no LSClient
	handler.ListInvoices(c)

	// Should return 500 since the UC is nil
	if w.Code != http.StatusInternalServerError {
		t.Fatalf("expected 500 when LSClient is nil, got %d", w.Code)
	}
}

func TestAdminBillingRefund_NoLSClient(t *testing.T) {
	handler, _ := setupAdminBillingHandler(t)

	gin.SetMode(gin.TestMode)
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/admin/billing/refund", http.NoBody)
	c.Request.Header.Set("Content-Type", "application/json")
	setAdminAuth(c)

	handler.ProcessRefund(c)

	// Should return 400 since there's no body
	if w.Code != http.StatusBadRequest {
		t.Fatalf("expected 400 with no body, got %d (body: %s)", w.Code, w.Body.String())
	}
}

func TestAdminBillingRevenueLinks(t *testing.T) {
	links := adminBillingRevenueLinks("90d")

	if links["self"].Href != "/api/v1/admin/billing/revenue?range=90d" {
		t.Errorf("unexpected self href: %s", links["self"].Href)
	}
	if _, ok := links["invoices"]; !ok {
		t.Error("missing invoices link")
	}
	if _, ok := links["dunning"]; !ok {
		t.Error("missing dunning link")
	}
}

func TestAdminBillingDunningLinks(t *testing.T) {
	links := adminBillingDunningLinks()

	if links["self"].Href != "/api/v1/admin/billing/dunning" {
		t.Errorf("unexpected self href: %s", links["self"].Href)
	}
	if _, ok := links["invoices"]; !ok {
		t.Error("missing invoices link")
	}
	if _, ok := links["revenue"]; !ok {
		t.Error("missing revenue link")
	}
}
