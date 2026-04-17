package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"testing"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/domain/entities"
)

func init() {
	gin.SetMode(gin.TestMode)
}

// fakeBackend records the incoming request so tests can assert tenant
// injection and auth forwarding behaviour.
type fakeBackend struct {
	method string
	path   string
	query  url.Values
	auth   string
	body   []byte
	respBy func(w http.ResponseWriter, r *http.Request)
}

func newFakeBackend(respBy func(w http.ResponseWriter, r *http.Request)) (*fakeBackend, *httptest.Server) {
	fb := &fakeBackend{respBy: respBy}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		body, _ := io.ReadAll(r.Body)
		fb.method = r.Method
		fb.path = r.URL.Path
		fb.query = r.URL.Query()
		fb.auth = r.Header.Get("Authorization")
		fb.body = body
		fb.respBy(w, r)
	}))
	return fb, srv
}

// newTestCP builds a minimal ControlPlane with only the pieces the delegation
// handlers use. It skips AuthMiddleware; tests call handlers directly with a
// pre-populated gin.Context to isolate delegation behaviour.
//
// The token signer is deterministic for test assertions — it echoes the
// inputs so tests can read tenant/user/role out of the forwarded Authorization
// header without JWT parsing.
func newTestCP(t *testing.T, coreURL, qsURL string) *ControlPlane {
	t.Helper()
	signer := func(userID, tenantID string, role entities.Role) (string, error) {
		return "test-token:" + userID + ":" + tenantID + ":" + string(role), nil
	}
	d, err := newDelegationClient(coreURL, qsURL, signer, http.DefaultClient)
	if err != nil {
		t.Fatalf("newDelegationClient: %v", err)
	}
	return &ControlPlane{delegation: d}
}

func callHandler(h gin.HandlerFunc, req *http.Request, tenantID string) *httptest.ResponseRecorder {
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)
	c.Request = req
	if tenantID != "" {
		c.Set("auth_tenant_id", tenantID)
		c.Set("auth_user_id", "user-"+tenantID)
		c.Set("auth_role", entities.RoleDeveloper)
	}
	h(c)
	return w
}

func TestProxyIngestSingle_InjectsTenantAndForwardsToCore(t *testing.T) {
	core, srv := newFakeBackend(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"event_id":"11111111-1111-1111-1111-111111111111","version":1}`))
	})
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	body := bytes.NewBufferString(`{"event_type":"order.created","entity_id":"o-1","payload":{"amount":10},"tenant_id":"spoofed"}`)
	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", body)
	w := callHandler(cp.ProxyIngestSingle, req, "tenant-real")

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if core.path != "/api/v1/events" {
		t.Errorf("upstream path: got %q, want /api/v1/events", core.path)
	}
	if core.auth != "Bearer test-token:user-tenant-real:tenant-real:developer" {
		t.Errorf("upstream auth: got %q, want per-request JWT scoped to caller", core.auth)
	}

	var forwarded map[string]any
	if err := json.Unmarshal(core.body, &forwarded); err != nil {
		t.Fatalf("decode forwarded body: %v", err)
	}
	if forwarded["tenant_id"] != "tenant-real" {
		t.Errorf("tenant_id: got %v, want tenant-real (spoofed client value must be overwritten)", forwarded["tenant_id"])
	}
	if forwarded["event_type"] != "order.created" {
		t.Errorf("event_type not forwarded: got %v", forwarded["event_type"])
	}
	if forwarded["entity_id"] != "o-1" {
		t.Errorf("entity_id not forwarded: got %v", forwarded["entity_id"])
	}
}

func TestProxyIngestSingle_NoTenantContext401(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", strings.NewReader(`{}`))
	w := callHandler(cp.ProxyIngestSingle, req, "")
	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: got %d, want 401", w.Code)
	}
}

func TestProxyIngestBatch_InjectsTenantOnEveryEvent(t *testing.T) {
	core, srv := newFakeBackend(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"ingested":2}`))
	})
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	batch := `{"events":[
		{"event_type":"a","entity_id":"e1","payload":{},"tenant_id":"spoof-a"},
		{"event_type":"b","entity_id":"e2","payload":{}}
	]}`
	req := httptest.NewRequest(http.MethodPost, "/api/v1/events/batch", strings.NewReader(batch))
	w := callHandler(cp.ProxyIngestBatch, req, "tenant-real")

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if core.path != "/api/v1/events/batch" {
		t.Errorf("upstream path: got %q, want /api/v1/events/batch", core.path)
	}
	var forwarded struct {
		Events []map[string]any `json:"events"`
	}
	if err := json.Unmarshal(core.body, &forwarded); err != nil {
		t.Fatalf("decode forwarded batch: %v", err)
	}
	if len(forwarded.Events) != 2 {
		t.Fatalf("events count: got %d, want 2", len(forwarded.Events))
	}
	for i, ev := range forwarded.Events {
		if ev["tenant_id"] != "tenant-real" {
			t.Errorf("events[%d].tenant_id: got %v, want tenant-real", i, ev["tenant_id"])
		}
	}
}

func TestProxyEventsQuery_InjectsTenantAsQueryParam(t *testing.T) {
	qs, srv := newFakeBackend(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"events":[],"count":0}`))
	})
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	// Client supplies a spoofed tenant_id plus legitimate since= filter.
	// Spoofed tenant must be overwritten; since= must be forwarded unchanged.
	req := httptest.NewRequest(http.MethodGet, "/api/v1/events/query?tenant_id=spoof&since=2026-04-17T00:00:00Z", nil)
	w := callHandler(cp.ProxyEventsQuery, req, "tenant-real")

	if w.Code != http.StatusOK {
		t.Fatalf("status: got %d, want 200 (body=%s)", w.Code, w.Body.String())
	}
	if qs.path != "/api/v1/events/query" {
		t.Errorf("upstream path: got %q, want /api/v1/events/query", qs.path)
	}
	if got := qs.query.Get("tenant_id"); got != "tenant-real" {
		t.Errorf("upstream tenant_id: got %q, want tenant-real", got)
	}
	if got := qs.query.Get("since"); got != "2026-04-17T00:00:00Z" {
		t.Errorf("upstream since: got %q, want original value", got)
	}
	if qs.auth != "Bearer test-token:user-tenant-real:tenant-real:developer" {
		t.Errorf("upstream auth: got %q, want per-request JWT scoped to caller", qs.auth)
	}
}

func TestProxyEventsQuery_NoTenantContext401(t *testing.T) {
	cp := newTestCP(t, "http://unused", "http://unused")
	req := httptest.NewRequest(http.MethodGet, "/api/v1/events/query", nil)
	w := callHandler(cp.ProxyEventsQuery, req, "")
	if w.Code != http.StatusUnauthorized {
		t.Errorf("status: got %d, want 401", w.Code)
	}
}

func TestDelegation_UpstreamErrorPropagates(t *testing.T) {
	_, srv := newFakeBackend(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusConflict)
		_, _ = w.Write([]byte(`{"error":"version conflict"}`))
	})
	defer srv.Close()
	cp := newTestCP(t, srv.URL, srv.URL)

	req := httptest.NewRequest(http.MethodPost, "/api/v1/events", strings.NewReader(`{"event_type":"x","entity_id":"e","payload":{}}`))
	w := callHandler(cp.ProxyIngestSingle, req, "tenant-real")

	if w.Code != http.StatusConflict {
		t.Errorf("status: got %d, want 409 propagated from upstream", w.Code)
	}
	if !strings.Contains(w.Body.String(), "version conflict") {
		t.Errorf("body: got %q, want upstream error to pass through", w.Body.String())
	}
}

// Claims import guard — without this, the entities/jwt packages look unused
// to the checker when other tests in this file are compiled in isolation.
var _ = entities.RoleDeveloper
var _ = jwt.SigningMethodHS256
