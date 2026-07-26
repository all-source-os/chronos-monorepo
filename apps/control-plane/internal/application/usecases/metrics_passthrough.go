package usecases

// Platform-metrics + cluster-status passthrough (ADMIN_TENANT_POWER_TOOL §3 Gap
// 2, §9 Phase 3 — CP half).
//
// WHY this exists: the admin /monitoring page (apps/admin) used to call the
// Query Service directly, cross-origin, with credentials:"include". The CP is
// Bearer-only via the admin BFF and the QS is a different origin with a
// different auth model, so the admin never authenticated → /monitoring was
// blank. The fix is a thin same-origin passthrough: the admin hits
// /api/v1/admin/metrics/* (BFF attaches the admin Bearer → AdminAuthMiddleware
// authorizes), and the CP re-fetches the QS metrics/cluster endpoints with its
// OWN long-lived service credential (the same admin/system JWT it already uses
// for Core; the QS :authenticated pipeline validates any HS256 JWT signed with
// the shared JWT_SECRET that carries exp + sub + tenant_id). The caller's cookie
// never reaches the QS.
//
// Shapes are returned UNCHANGED for summary + timeseries so the admin client and
// its charts work without reshaping. Cluster members are the one mapping: the QS
// /api/cluster/members payload ({node, self, connected}) does not match the
// admin's ClusterMember shape ({id, role, address, status, …}), so we map it
// into the admin shape here (documented divergence — see FetchClusterMembers).
//
// QS-unreachable discipline (§Resilience): every fetch that fails for any reason
// returns a clean zeroed/empty value, NEVER an error. The handler always renders
// a zero state, never a 500 — a blank dashboard from an upstream blip is worse
// than a zero one.

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/url"
	"strings"

	"github.com/allsource/control-plane/internal/domain/repositories"
)

// --- Response shapes (match apps/admin/src/lib/metrics-api.ts exactly) ---

// MetricsSummary mirrors the QS GET /api/admin/metrics/summary body and the
// admin MetricsSummary interface field-for-field.
type MetricsSummary struct {
	UptimeSeconds     float64 `json:"uptime_seconds"`
	EventsTotal       float64 `json:"events_total"`
	EventsPerSecond   float64 `json:"events_per_second"`
	QueryLatencyP99Ms float64 `json:"query_latency_p99_ms"`
	ErrorRatePercent  float64 `json:"error_rate_percent"`
	ActiveTenants     float64 `json:"active_tenants"`
}

// TimeseriesPoint mirrors the admin TimeseriesPoint interface.
type TimeseriesPoint struct {
	Timestamp string  `json:"timestamp"`
	Value     float64 `json:"value"`
}

// TimeseriesResponse mirrors the QS GET /api/admin/metrics/timeseries body
// ({metric, range, points}). The admin client reads `.points` (fetchTimeseries:
// `data.points || data`), so the envelope is returned verbatim.
type TimeseriesResponse struct {
	Metric string            `json:"metric"`
	Range  string            `json:"range"`
	Points []TimeseriesPoint `json:"points"`
}

// ClusterMember mirrors the admin ClusterMember interface
// (apps/admin/src/lib/metrics-api.ts). Note this is NOT the QS member shape —
// see FetchClusterMembers for the mapping rationale.
type ClusterMember struct {
	Role          string `json:"role"` // "leader" | "follower"
	ID            string `json:"id"`
	Address       string `json:"address"`
	Status        string `json:"status"` // "healthy" | "degraded" | "unreachable"
	LagMs         *int64 `json:"lag_ms,omitempty"`
	UptimeSeconds *int64 `json:"uptime_seconds,omitempty"`
}

// --- QS-side wire shapes we decode before mapping ---

// qsClusterMembersResponse is the QS GET /api/cluster/members body:
//
//	{"data": {"members": [{"node": "...", "self": true, "connected": true}],
//	          "total": 1, "strategy": "..."}}
//
// (ClusterController.members, apps/query-service). We decode it and map each
// member into the admin ClusterMember shape.
type qsClusterMembersResponse struct {
	Data struct {
		Members []struct {
			Node      string `json:"node"`
			Self      bool   `json:"self"`
			Connected bool   `json:"connected"`
		} `json:"members"`
		Total    int    `json:"total"`
		Strategy string `json:"strategy"`
	} `json:"data"`
}

// MetricsPassthroughUseCase fetches platform metrics + cluster status from the
// Query Service on behalf of an authenticated admin, using the CP's own service
// credential. It owns no state beyond its HTTP client, the QS base URL, and the
// bearer token.
type MetricsPassthroughUseCase struct {
	httpClient   *http.Client
	queryBaseURL string // e.g. http://allsource-query.internal:3902 (no trailing slash)
	serviceToken string // long-lived admin/system JWT the QS :authenticated pipeline accepts
	// tenantRepo backs the active_tenants overlay. Core's exporter emits no
	// allsource_active_tenants series, so the QS summary always reports 0 — the
	// CP has the authoritative tenant list, so we count active tenants here. nil
	// disables the overlay (active_tenants stays whatever the QS returned).
	tenantRepo repositories.TenantRepository
}

// NewMetricsPassthroughUseCase builds the passthrough. A nil httpClient defaults
// to http.DefaultClient. An empty queryBaseURL or serviceToken disables the
// upstream call — every fetch then returns the zero/empty shape (so the admin
// renders a zero state in local/unconfigured environments instead of erroring).
func NewMetricsPassthroughUseCase(httpClient *http.Client, queryBaseURL, serviceToken string, tenantRepo repositories.TenantRepository) *MetricsPassthroughUseCase {
	if httpClient == nil {
		httpClient = http.DefaultClient
	}
	return &MetricsPassthroughUseCase{
		httpClient:   httpClient,
		queryBaseURL: strings.TrimRight(queryBaseURL, "/"),
		serviceToken: serviceToken,
		tenantRepo:   tenantRepo,
	}
}

// FetchSummary returns the QS metrics summary verbatim. On any failure
// (unconfigured, unreachable, non-2xx, decode error) it returns a zeroed
// MetricsSummary so the admin renders a zero state, never a 500.
func (uc *MetricsPassthroughUseCase) FetchSummary(ctx context.Context) MetricsSummary {
	var out MetricsSummary
	if uc.queryBaseURL != "" && uc.serviceToken != "" {
		if body, ok := uc.get(ctx, "/api/admin/metrics/summary", nil); ok {
			if err := json.Unmarshal(body, &out); err != nil {
				out = MetricsSummary{}
			}
		}
	}
	// active_tenants has no Core/QS series (the exporter emits none), so the QS
	// summary always reports 0 even with active tenants. Overlay the real count
	// from the CP's own tenant list. Guarded on ==0 so a future real QS series
	// wins, and computed even when the QS is unreachable (it's CP-sourced now).
	if out.ActiveTenants == 0 {
		if n, ok := uc.activeTenantCount(); ok {
			out.ActiveTenants = n
		}
	}
	return out
}

// activeTenantCount returns the number of active-status tenants from the CP's
// tenant repository. Returns (0,false) when no repo is wired or the lookup
// fails, so the caller leaves active_tenants untouched rather than forcing a 0.
func (uc *MetricsPassthroughUseCase) activeTenantCount() (float64, bool) {
	if uc.tenantRepo == nil {
		return 0, false
	}
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return 0, false
	}
	var n float64
	for _, t := range tenants {
		if t != nil && t.IsActive() {
			n++
		}
	}
	return n, true
}

// FetchTimeseries returns the QS timeseries envelope verbatim ({metric, range,
// points}). On any failure it returns an empty-points envelope echoing the
// requested metric + range, so the chart renders an empty series rather than
// crashing or 500ing.
func (uc *MetricsPassthroughUseCase) FetchTimeseries(ctx context.Context, metric, timeRange string) TimeseriesResponse {
	empty := TimeseriesResponse{Metric: metric, Range: timeRange, Points: []TimeseriesPoint{}}
	if uc.queryBaseURL == "" || uc.serviceToken == "" {
		return empty
	}
	q := url.Values{}
	q.Set("metric", metric)
	q.Set("range", timeRange)
	body, ok := uc.get(ctx, "/api/admin/metrics/timeseries", q)
	if !ok {
		return empty
	}
	var out TimeseriesResponse
	if err := json.Unmarshal(body, &out); err != nil {
		return empty
	}
	if out.Points == nil {
		out.Points = []TimeseriesPoint{}
	}
	// Preserve the caller's requested metric/range if the QS omitted them.
	if out.Metric == "" {
		out.Metric = metric
	}
	if out.Range == "" {
		out.Range = timeRange
	}
	return out
}

// FetchClusterMembers returns the cluster membership in the admin ClusterMember
// shape. On any failure it returns an empty slice (never nil, never an error) so
// the cluster panel shows "No cluster members found." rather than crashing.
//
// DIVERGENCE (documented): the QS /api/cluster/members payload is
// {node, self, connected} (BEAM node membership), which does NOT carry the
// admin's leader/follower role, address, lag, or uptime. We map it into the
// admin ClusterMember shape so the existing component renders:
//   - id      ← node name
//   - role    ← "leader" when self (the QS reports its own node), else "follower"
//   - address ← node name (the BEAM node name is the only address QS exposes)
//   - status  ← "healthy" when connected, else "unreachable"
//   - lag_ms / uptime_seconds ← omitted (QS membership does not report them)
//
// This keeps the passthrough BFF-consistent; a richer leader/follower view would
// require Core replication metadata the QS membership endpoint does not surface.
func (uc *MetricsPassthroughUseCase) FetchClusterMembers(ctx context.Context) []ClusterMember {
	out := []ClusterMember{}
	if uc.queryBaseURL == "" || uc.serviceToken == "" {
		return out
	}
	body, ok := uc.get(ctx, "/api/cluster/members", nil)
	if !ok {
		return out
	}
	var qs qsClusterMembersResponse
	if err := json.Unmarshal(body, &qs); err != nil {
		return out
	}
	for _, m := range qs.Data.Members {
		role := "follower"
		if m.Self {
			role = "leader"
		}
		status := "unreachable"
		if m.Connected {
			status = "healthy"
		}
		out = append(out, ClusterMember{
			ID:      m.Node,
			Role:    role,
			Address: m.Node,
			Status:  status,
		})
	}
	return out
}

// get performs an authenticated GET against the QS and returns the raw body on a
// 2xx. Any transport error, non-2xx status, or body-read error returns
// (nil, false) so callers fall back to their zero/empty shape. It never returns
// an error — the zero state IS the error handling.
func (uc *MetricsPassthroughUseCase) get(ctx context.Context, path string, query url.Values) ([]byte, bool) {
	target := uc.queryBaseURL + path
	if len(query) > 0 {
		target += "?" + query.Encode()
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, target, http.NoBody)
	if err != nil {
		return nil, false
	}
	req.Header.Set("Authorization", "Bearer "+uc.serviceToken)
	req.Header.Set("Accept", "application/json")

	resp, err := uc.httpClient.Do(req)
	if err != nil {
		return nil, false
	}
	defer func() { _ = resp.Body.Close() }() //nolint:errcheck // close-on-defer, non-actionable
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		// Drain so the connection can be reused, then fall back to zero state.
		_, _ = io.Copy(io.Discard, resp.Body)
		return nil, false
	}
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, false
	}
	return body, true
}
