package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// Tests for the tenant-scoped analytics surface added in #230.
//
// Core's /api/v1/stats and /api/v1/entities/{id}/state are GLOBAL unless a
// tenant_id is supplied, so the security property under test is that each proxy
// FORCES the auth-derived tenant and overwrites anything the caller supplied.

func okJSON(body string) func(w http.ResponseWriter, r *http.Request) {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(body)) //nolint:errcheck // test response
	}
}

func TestProxyStats_ForcesAuthDerivedTenant(t *testing.T) {
	core, srv := newFakeBackend(okJSON(`{"total_events":3,"total_entities":2}`))
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	// Caller tries to read another tenant's totals.
	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/stats?tenant_id=victim-tenant", http.NoBody)
	w := callHandler(cp.ProxyStats, req, "tenant-real")

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if core.path != "/api/v1/stats" {
		t.Errorf("upstream path: got %q, want /api/v1/stats", core.path)
	}
	if got := core.query.Get("tenant_id"); got != "tenant-real" {
		t.Errorf("upstream tenant_id: got %q, want tenant-real (spoof must be overwritten)", got)
	}
	// A single value, not the caller's appended alongside ours.
	if vals := core.query["tenant_id"]; len(vals) != 1 {
		t.Errorf("upstream tenant_id values: got %v, want exactly one", vals)
	}
}

func TestProxyStats_NeverForwardsUnscoped(t *testing.T) {
	core, srv := newFakeBackend(okJSON(`{}`))
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/stats", http.NoBody)
	callHandler(cp.ProxyStats, req, "tenant-real")

	// Absent tenant_id means whole-store totals at Core, so the proxy must
	// always supply one.
	if core.query.Get("tenant_id") == "" {
		t.Error("upstream request carried no tenant_id — that would return global, cross-tenant totals")
	}
}

func TestProxyStats_NoTenantContext401(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet, "/api/v1/stats", http.NoBody)
	w := callHandler(cp.ProxyStats, req, "")
	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: got %d, want 401", w.Code)
	}
}

func TestProxyEntityState_ForcesTenantAndForwardsEntity(t *testing.T) {
	core, srv := newFakeBackend(okJSON(`{"entity_id":"e1","current_state":{}}`))
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	req := httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/entities/e1/state?tenant_id=victim-tenant&as_of=2026-07-01T00:00:00Z", http.NoBody)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = req
	c.Params = gin.Params{{Key: "entity_id", Value: "e1"}}
	c.Set("auth_tenant_id", "tenant-real")
	c.Set("auth_user_id", "user-tenant-real")
	c.Set("auth_role", entities.RoleDeveloper)
	cp.ProxyEntityState(c)

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if core.path != "/api/v1/entities/e1/state" {
		t.Errorf("upstream path: got %q, want /api/v1/entities/e1/state", core.path)
	}
	if got := core.query.Get("tenant_id"); got != "tenant-real" {
		t.Errorf("upstream tenant_id: got %q, want tenant-real", got)
	}
	if got := core.query.Get("as_of"); got != "2026-07-01T00:00:00Z" {
		t.Errorf("upstream as_of: got %q, want the caller's value forwarded", got)
	}
}

func TestProxyEntitySnapshot_ForcesTenant(t *testing.T) {
	core, srv := newFakeBackend(okJSON(`{"entity_id":"e1"}`))
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/entities/e1/snapshot", http.NoBody)
	c.Params = gin.Params{{Key: "entity_id", Value: "e1"}}
	c.Set("auth_tenant_id", "tenant-real")
	c.Set("auth_user_id", "user-tenant-real")
	c.Set("auth_role", entities.RoleDeveloper)
	cp.ProxyEntitySnapshot(c)

	if core.path != "/api/v1/entities/e1/snapshot" {
		t.Errorf("upstream path: got %q, want /api/v1/entities/e1/snapshot", core.path)
	}
	if got := core.query.Get("tenant_id"); got != "tenant-real" {
		t.Errorf("upstream tenant_id: got %q, want tenant-real", got)
	}
}

func TestProxyEntityState_EscapesEntityID(t *testing.T) {
	core, srv := newFakeBackend(okJSON(`{}`))
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	// An entity_id containing a slash must not let the caller reach a
	// different Core path.
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/entities/x/snapshot", http.NoBody)
	c.Params = gin.Params{{Key: "entity_id", Value: "../../stats"}}
	c.Set("auth_tenant_id", "tenant-real")
	c.Set("auth_user_id", "user-tenant-real")
	c.Set("auth_role", entities.RoleDeveloper)
	cp.ProxyEntityState(c)

	if core.path == "/api/v1/stats" {
		t.Errorf("path traversal reached %q — entity_id must be escaped", core.path)
	}
}

func TestProxyEntityState_NoTenantContext401(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/entities/e1/state", http.NoBody)
	c.Params = gin.Params{{Key: "entity_id", Value: "e1"}}
	cp.ProxyEntityState(c)

	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: got %d, want 401", w.Code)
	}
}

func TestProxyEntityState_MissingEntityID400(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = httptest.NewRequestWithContext(context.Background(), http.MethodGet,
		"/api/v1/entities//state", http.NoBody)
	c.Set("auth_tenant_id", "tenant-real")
	c.Set("auth_user_id", "user-tenant-real")
	c.Set("auth_role", entities.RoleDeveloper)
	cp.ProxyEntityState(c)

	if w.Code != http.StatusBadRequest {
		t.Errorf("status: got %d, want 400", w.Code)
	}
}
