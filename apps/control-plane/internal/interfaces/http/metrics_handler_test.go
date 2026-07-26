package http

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// metricsRouter builds the real admin router for the metrics passthrough with
// AdminAuthMiddleware, so 401/403 behavior is exercised end-to-end (proves the
// /api/v1/admin group middleware reuse — no new auth code). The routes mirror
// main.go registration exactly so prompt 035's admin client matches.
func metricsRouter(uc *usecases.MetricsPassthroughUseCase) *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	h := NewMetricsHandler(uc)
	admin := r.Group("/api/v1/admin")
	admin.Use(AdminAuthMiddleware(fleetTestJWTSecret))
	m := admin.Group("/metrics")
	m.GET("/summary", h.Summary)
	m.GET("/timeseries", h.Timeseries)
	admin.GET("/cluster/members", h.ClusterMembers)
	return r
}

// fakeQS stands up a minimal Query Service that serves the three endpoints the
// passthrough fetches, asserting the CP authenticated with a Bearer token (the
// service credential — NOT the admin's cookie). It records whether each route was
// hit and with what Authorization header.
type fakeQS struct {
	srv      *httptest.Server
	gotAuth  string
	gotPaths []string
}

func newFakeQS(t *testing.T) *fakeQS {
	t.Helper()
	f := &fakeQS{}
	mux := http.NewServeMux()
	mux.HandleFunc("/api/admin/metrics/summary", func(w http.ResponseWriter, r *http.Request) {
		f.gotAuth = r.Header.Get("Authorization")
		f.gotPaths = append(f.gotPaths, r.URL.Path)
		_ = json.NewEncoder(w).Encode(map[string]any{ //nolint:errcheck // test response
			"uptime_seconds":       86400.0,
			"events_total":         1500000.0,
			"events_per_second":    469.0,
			"query_latency_p99_ms": 11.9,
			"error_rate_percent":   0.02,
			"active_tenants":       12.0,
		})
	})
	mux.HandleFunc("/api/admin/metrics/timeseries", func(w http.ResponseWriter, r *http.Request) {
		f.gotPaths = append(f.gotPaths, r.URL.Path+"?"+r.URL.RawQuery)
		_ = json.NewEncoder(w).Encode(map[string]any{ //nolint:errcheck // test response
			"metric": r.URL.Query().Get("metric"),
			"range":  r.URL.Query().Get("range"),
			"points": []map[string]any{
				{"timestamp": "2026-06-25T12:00:00Z", "value": 469.0},
				{"timestamp": "2026-06-25T12:01:00Z", "value": 470.5},
			},
		})
	})
	mux.HandleFunc("/api/cluster/members", func(w http.ResponseWriter, r *http.Request) {
		f.gotPaths = append(f.gotPaths, r.URL.Path)
		// The real QS member shape ({node, self, connected}) wrapped in {data:…}.
		_ = json.NewEncoder(w).Encode(map[string]any{
			"data": map[string]any{
				"members": []map[string]any{
					{"node": "core-leader@host", "self": true, "connected": true},
					{"node": "core-follower-1@host", "self": false, "connected": true},
					{"node": "core-follower-2@host", "self": false, "connected": false},
				},
				"total":    3,
				"strategy": "gossip",
			},
		})
	})
	f.srv = httptest.NewServer(mux)
	t.Cleanup(f.srv.Close)
	return f
}

// --- §9 Phase 3 acceptance ---

// Summary returns the QS MetricsSummary verbatim for an admin JWT, and the CP
// reached the QS with its OWN service token (not the caller's).
func TestMetrics_SummaryReturnsQSShape(t *testing.T) {
	qs := newFakeQS(t)
	uc := usecases.NewMetricsPassthroughUseCase(qs.srv.Client(), qs.srv.URL, "service-token-xyz", nil)
	r := metricsRouter(uc)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/summary", adminToken(t, entities.RoleAdmin), nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var got usecases.MetricsSummary
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if got.EventsPerSecond != 469.0 || got.ActiveTenants != 12 || got.UptimeSeconds != 86400 {
		t.Errorf("summary mismatch: %+v", got)
	}
	if got.QueryLatencyP99Ms != 11.9 || got.ErrorRatePercent != 0.02 || got.EventsTotal != 1500000 {
		t.Errorf("summary mismatch: %+v", got)
	}
	// The CP must authenticate to the QS with the service token, not a cookie.
	if qs.gotAuth != "Bearer service-token-xyz" {
		t.Errorf("QS Authorization = %q, want Bearer service-token-xyz", qs.gotAuth)
	}
}

// Timeseries returns the QS envelope verbatim ({metric, range, points}) and
// forwards the metric+range query params.
func TestMetrics_TimeseriesReturnsQSEnvelope(t *testing.T) {
	qs := newFakeQS(t)
	uc := usecases.NewMetricsPassthroughUseCase(qs.srv.Client(), qs.srv.URL, "svc", nil)
	r := metricsRouter(uc)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/timeseries?metric=error_rate_percent&range=24h", adminToken(t, entities.RoleAdmin), nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var got usecases.TimeseriesResponse
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if got.Metric != "error_rate_percent" || got.Range != "24h" {
		t.Errorf("envelope metric/range = %q/%q, want error_rate_percent/24h", got.Metric, got.Range)
	}
	if len(got.Points) != 2 || got.Points[0].Value != 469.0 || got.Points[0].Timestamp == "" {
		t.Errorf("points mismatch: %+v", got.Points)
	}
}

// ClusterMembers maps the QS member shape into the admin ClusterMember shape and
// wraps it in {members:[…]} (what fetchClusterMembers' `data.members` reads).
func TestMetrics_ClusterMembersMapped(t *testing.T) {
	qs := newFakeQS(t)
	uc := usecases.NewMetricsPassthroughUseCase(qs.srv.Client(), qs.srv.URL, "svc", nil)
	r := metricsRouter(uc)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/cluster/members", adminToken(t, entities.RoleAdmin), nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var got struct {
		Members []usecases.ClusterMember `json:"members"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(got.Members) != 3 {
		t.Fatalf("expected 3 members, got %d (%+v)", len(got.Members), got.Members)
	}
	// self → leader+healthy; connected follower → follower+healthy; disconnected → unreachable.
	if got.Members[0].Role != "leader" || got.Members[0].Status != "healthy" || got.Members[0].ID != "core-leader@host" {
		t.Errorf("member[0] = %+v, want leader/healthy/core-leader@host", got.Members[0])
	}
	if got.Members[1].Role != "follower" || got.Members[1].Status != "healthy" {
		t.Errorf("member[1] = %+v, want follower/healthy", got.Members[1])
	}
	if got.Members[2].Status != "unreachable" {
		t.Errorf("member[2] status = %q, want unreachable", got.Members[2].Status)
	}
}

// Every metrics endpoint returns 401 without a token and 403 with a non-admin
// role (proves AdminAuthMiddleware reuse — same gate as the rest of /admin).
func TestMetrics_403WithoutAdmin(t *testing.T) {
	qs := newFakeQS(t)
	uc := usecases.NewMetricsPassthroughUseCase(qs.srv.Client(), qs.srv.URL, "svc", nil)
	r := metricsRouter(uc)

	paths := []string{
		"/api/v1/admin/metrics/summary",
		"/api/v1/admin/metrics/timeseries",
		"/api/v1/admin/cluster/members",
	}
	for _, p := range paths {
		t.Run(p, func(t *testing.T) {
			// No token → 401.
			w := doAdmin(t, r, http.MethodGet, p, "", nil)
			if w.Code != http.StatusUnauthorized {
				t.Errorf("no token: expected 401, got %d", w.Code)
			}
			// Non-admin role → 403.
			w = doAdmin(t, r, http.MethodGet, p, adminToken(t, entities.RoleDeveloper), nil)
			if w.Code != http.StatusForbidden {
				t.Errorf("developer role: expected 403, got %d (%s)", w.Code, w.Body.String())
			}
		})
	}
}

// QS-unreachable ⇒ clean zeroed/empty payloads with HTTP 200 (never a 500), so
// the admin renders a zero state. Uses a base URL that fails to connect.
func TestMetrics_QSUnreachableZeroState(t *testing.T) {
	// Point at a closed port: every fetch errors at the transport layer.
	uc := usecases.NewMetricsPassthroughUseCase(http.DefaultClient, "http://127.0.0.1:1", "svc", nil)
	r := metricsRouter(uc)
	tok := adminToken(t, entities.RoleAdmin)

	// summary → zeroed, 200.
	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/summary", tok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("summary: expected 200, got %d (%s)", w.Code, w.Body.String())
	}
	var sum usecases.MetricsSummary
	if err := json.Unmarshal(w.Body.Bytes(), &sum); err != nil {
		t.Fatalf("summary unmarshal: %v", err)
	}
	if sum != (usecases.MetricsSummary{}) {
		t.Errorf("expected zeroed summary, got %+v", sum)
	}

	// timeseries → empty points (non-nil), echoes requested metric/range, 200.
	w = doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/timeseries?metric=events_per_second&range=1h", tok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("timeseries: expected 200, got %d", w.Code)
	}
	var ts usecases.TimeseriesResponse
	if err := json.Unmarshal(w.Body.Bytes(), &ts); err != nil {
		t.Fatalf("timeseries unmarshal: %v", err)
	}
	if ts.Points == nil || len(ts.Points) != 0 {
		t.Errorf("expected empty (non-nil) points, got %+v", ts.Points)
	}
	if ts.Metric != "events_per_second" || ts.Range != "1h" {
		t.Errorf("expected echoed metric/range, got %q/%q", ts.Metric, ts.Range)
	}

	// cluster members → {members:[]} (non-nil empty array, renders in JSON as []), 200.
	w = doAdmin(t, r, http.MethodGet, "/api/v1/admin/cluster/members", tok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("cluster: expected 200, got %d", w.Code)
	}
	if body := w.Body.String(); body != `{"members":[]}` {
		t.Errorf("expected {\"members\":[]}, got %s", body)
	}
}

// A QS that 5xxs (reachable but erroring) also yields the zero state, not a 500.
func TestMetrics_QSErrorStatusZeroState(t *testing.T) {
	bad := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusBadGateway)
		_, _ = w.Write([]byte(`{"error":"boom"}`)) //nolint:errcheck // test response
	}))
	t.Cleanup(bad.Close)

	uc := usecases.NewMetricsPassthroughUseCase(bad.Client(), bad.URL, "svc", nil)
	r := metricsRouter(uc)
	tok := adminToken(t, entities.RoleAdmin)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/summary", tok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200 on QS 502, got %d (%s)", w.Code, w.Body.String())
	}
	var sum usecases.MetricsSummary
	if err := json.Unmarshal(w.Body.Bytes(), &sum); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if sum != (usecases.MetricsSummary{}) {
		t.Errorf("expected zeroed summary on QS 502, got %+v", sum)
	}
}

// When the passthrough is unconfigured (empty base URL / token), it short-circuits
// to the zero state without any network call — the local/dev default.
func TestMetrics_UnconfiguredZeroState(t *testing.T) {
	uc := usecases.NewMetricsPassthroughUseCase(nil, "", "", nil)
	r := metricsRouter(uc)
	tok := adminToken(t, entities.RoleAdmin)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/cluster/members", tok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	if body := w.Body.String(); body != `{"members":[]}` {
		t.Errorf("expected {\"members\":[]}, got %s", body)
	}
}

// Core's exporter emits no allsource_active_tenants series, so the QS summary
// reports 0. The CP overlays the real count from its own tenant repo — counting
// only active-status tenants — and does so even when the QS is unreachable
// (the summary is otherwise zero). 2 active + 1 suspended ⇒ active_tenants == 2.
func TestMetrics_SummaryOverlaysActiveTenantsFromRepo(t *testing.T) {
	repo := persistence.NewMemoryTenantRepository()
	for _, tt := range []struct {
		id     string
		status entities.TenantStatus
	}{
		{"a", entities.TenantStatusActive},
		{"b", entities.TenantStatusActive},
		{"c", entities.TenantStatusSuspended},
	} {
		if err := repo.Save(&entities.Tenant{ID: tt.id, Name: tt.id, Status: tt.status}); err != nil {
			t.Fatalf("seed %s: %v", tt.id, err)
		}
	}

	// Point at a closed port: the QS summary fetch fails, so events_total etc.
	// stay 0 — but active_tenants must still reflect the CP's real count.
	uc := usecases.NewMetricsPassthroughUseCase(http.DefaultClient, "http://127.0.0.1:1", "svc", repo)
	r := metricsRouter(uc)

	w := doAdmin(t, r, http.MethodGet, "/api/v1/admin/metrics/summary", adminToken(t, entities.RoleAdmin), nil)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
	var got usecases.MetricsSummary
	if err := json.Unmarshal(w.Body.Bytes(), &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if got.ActiveTenants != 2 {
		t.Errorf("active_tenants overlay = %v, want 2", got.ActiveTenants)
	}
}
