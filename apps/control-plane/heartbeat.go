package main

// Event-sourced heartbeats — the canonical status signal for the fleet.
//
// Control Plane is the single natural host for this because (a) it already
// has internal-network access to every backend, (b) it already writes events
// to Core, and (c) its own uptime is a superset of the status page's
// usefulness — if CP is down, nothing else can report anyway.
//
// Every heartbeat probes a backend, emits a `service.heartbeat` event into
// Core under tenant=system, and is recorded permanently. The status endpoint
// then folds those events into a current-view projection. Time-travel,
// uptime history, and "who was down when?" all come for free.

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// heartbeatInterval is how often we probe each service. Chosen to be tight
// enough that the status page feels live but loose enough that we don't
// flood Core with low-value writes. At 10s × 8 services = 48 events/min.
const heartbeatInterval = 10 * time.Second

// heartbeatTTL is how long a single heartbeat counts as "fresh" when the
// status endpoint aggregates. Slightly larger than 2× the interval so one
// dropped probe doesn't flip a service red.
const heartbeatTTL = 25 * time.Second

// heartbeatTenant is where service heartbeats live. Not user data; a
// dedicated system tenant keeps it out of per-user queries and lets us
// set retention/compaction independently later.
const heartbeatTenant = "system"

// heartbeatEventType is the event type name. Consumers (status page,
// alerting, graphs) filter on this.
const heartbeatEventType = "service.heartbeat"

// probe describes one backend the Control Plane checks. Method defaults to
// GET; bodyMatch (optional) is a substring the response body must contain
// to be considered healthy. HEAD is avoided entirely — some backends (Gin
// with default setup) don't handle HEAD for paths that only registered GET,
// which produced false-dead signals before this system replaced Vigil.
type probe struct {
	name      string
	url       string
	bodyMatch string
}

// probesFromEnv builds the probe list. Targets default to Fly internal
// hostnames for the services on the internal network, and public URLs for
// services that are publicly reachable (auth, registry, web).
//
// All backend URLs honour env overrides to keep this configurable without
// a rebuild: CORE_HEALTH_URL, QUERY_HEALTH_URL, PRIME_HEALTH_URL,
// AUTH_HEALTH_URL, REGISTRY_HEALTH_URL, WEB_HEALTH_URL.
func probesFromEnv(getEnv func(string) string) []probe {
	probes := []probe{
		// Control Plane probes its own loopback so a stuck process still
		// emits "unhealthy" before it misses too many intervals.
		{
			name:      "control-plane",
			url:       "http://127.0.0.1:" + DefaultPort + "/health",
			bodyMatch: `"status":"healthy"`,
		},
	}

	add := func(name, envVar, fallback, bodyMatch string) {
		u := getEnv(envVar)
		if u == "" {
			u = fallback
		}
		probes = append(probes, probe{name: name, url: u, bodyMatch: bodyMatch})
	}

	// Internal .fly network — the delegation-layer backends.
	add("core", "CORE_HEALTH_URL", "http://allsource-core.internal:3900/health", `"status":"healthy"`)
	add("query", "QUERY_HEALTH_URL", "http://allsource-query.internal:3902/health", "")
	add("prime", "PRIME_HEALTH_URL", "http://allsource-prime.internal:3905/health", `"status":"ok"`)

	// Publicly-reachable services (keep probing over public DNS so we catch
	// cert/edge issues that internal routing would hide).
	add("auth", "AUTH_HEALTH_URL", "https://allsource-auth.fly.dev/health", "")
	add("registry", "REGISTRY_HEALTH_URL", "https://allsource-registry.fly.dev/health", "")
	add("web", "WEB_HEALTH_URL", "https://all-source.xyz/", "")

	return probes
}

// heartbeatEmitter owns the probe loop + Core client.
type heartbeatEmitter struct {
	probes     []probe
	coreClient clients.CoreClient
	httpClient *http.Client
}

func newHeartbeatEmitter(coreClient clients.CoreClient, httpClient *http.Client, probes []probe) *heartbeatEmitter {
	if httpClient == nil {
		httpClient = &http.Client{Timeout: 4 * time.Second}
	}
	return &heartbeatEmitter{
		probes:     probes,
		coreClient: coreClient,
		httpClient: httpClient,
	}
}

// run ticks forever (until ctx cancels), emitting one heartbeat event per
// probe per interval. Probes run concurrently so a slow backend doesn't
// starve the others.
func (h *heartbeatEmitter) run(ctx context.Context, interval time.Duration) {
	// Fire once immediately so the status page has data on startup.
	h.emitAll(ctx)

	t := time.NewTicker(interval)
	defer t.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-t.C:
			h.emitAll(ctx)
		}
	}
}

func (h *heartbeatEmitter) emitAll(ctx context.Context) {
	var wg sync.WaitGroup
	for _, p := range h.probes {
		wg.Add(1)
		go func(p probe) {
			defer wg.Done()
			h.probeAndEmit(ctx, p)
		}(p)
	}
	wg.Wait()
}

// probeAndEmit runs one HTTP probe and writes the result to Core regardless
// of outcome — a heartbeat event is emitted in both healthy and unhealthy
// cases so consumers can distinguish "last seen healthy" from "probe never
// ran".
func (h *heartbeatEmitter) probeAndEmit(ctx context.Context, p probe) {
	started := time.Now()
	status, errMsg := h.probeOnce(ctx, p)
	latency := time.Since(started)

	payload := map[string]any{
		"service":    p.name,
		"status":     status,
		"latency_ms": latency.Milliseconds(),
		"probed_url": p.url,
	}
	if errMsg != "" {
		payload["error"] = errMsg
	}

	// Best effort — if Core is down we can't emit; the projection will show
	// "stale" for this service, which is itself the right signal.
	_, _ = h.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: heartbeatEventType,
		EntityID:  p.name,
		TenantID:  heartbeatTenant,
		Payload:   payload,
	})
}

func (h *heartbeatEmitter) probeOnce(ctx context.Context, p probe) (status string, errMsg string) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, p.url, nil)
	if err != nil {
		return "unhealthy", "build request: " + err.Error()
	}
	resp, err := h.httpClient.Do(req)
	if err != nil {
		return "unhealthy", "request: " + err.Error()
	}
	defer func() { _ = resp.Body.Close() }()

	if resp.StatusCode < 200 || resp.StatusCode >= 400 {
		return "unhealthy", fmt.Sprintf("http %d", resp.StatusCode)
	}
	if p.bodyMatch != "" {
		body, err := io.ReadAll(io.LimitReader(resp.Body, 8192))
		if err != nil {
			return "unhealthy", "read body: " + err.Error()
		}
		if !bytes.Contains(body, []byte(p.bodyMatch)) {
			return "unhealthy", "body missing expected marker"
		}
	}
	return "healthy", ""
}

// --------------- Status projection ---------------

// serviceStatus is the current-view shape returned by /api/v1/status/services.
type serviceStatus struct {
	Service    string  `json:"service"`
	Status     string  `json:"status"`      // "healthy" | "unhealthy" | "stale"
	LatencyMs  int64   `json:"latency_ms"`  // from the last heartbeat
	LastSeen   string  `json:"last_seen"`   // RFC3339 of last heartbeat received
	AgeSeconds float64 `json:"age_seconds"` // seconds since last heartbeat
	Error      string  `json:"error,omitempty"`
	ProbedURL  string  `json:"probed_url,omitempty"`
}

// projectStatus folds the recent heartbeat events into one entry per
// service, picking the newest heartbeat and flagging anything older than
// heartbeatTTL as "stale".
func projectStatus(events []clients.EventEntry, now time.Time) []serviceStatus {
	// newest heartbeat per entity_id
	latest := map[string]clients.EventEntry{}
	for _, e := range events {
		if e.EventType != heartbeatEventType {
			continue
		}
		if prev, ok := latest[e.EntityID]; ok {
			if e.Timestamp <= prev.Timestamp {
				continue
			}
		}
		latest[e.EntityID] = e
	}

	out := make([]serviceStatus, 0, len(latest))
	for _, e := range latest {
		s := serviceStatus{Service: e.EntityID, LastSeen: e.Timestamp}
		ts, err := time.Parse(time.RFC3339Nano, e.Timestamp)
		if err != nil {
			ts, _ = time.Parse(time.RFC3339, e.Timestamp)
		}
		age := now.Sub(ts)
		s.AgeSeconds = age.Seconds()

		reported, _ := e.Payload["status"].(string)
		if age > heartbeatTTL {
			s.Status = "stale"
		} else {
			s.Status = reported
		}
		if v, ok := e.Payload["latency_ms"].(float64); ok {
			s.LatencyMs = int64(v)
		}
		if v, ok := e.Payload["error"].(string); ok {
			s.Error = v
		}
		if v, ok := e.Payload["probed_url"].(string); ok {
			s.ProbedURL = v
		}
		out = append(out, s)
	}

	sort.Slice(out, func(i, j int) bool { return out[i].Service < out[j].Service })
	return out
}

// statusServicesHandler returns the JSON projection. Powers the public
// status page frontend; also a stable feed for external dashboards/alerts.
func (cp *ControlPlane) statusServicesHandler(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"services": cp.currentServiceStatus()})
}

// statusPageHandler renders the HTML status page using the same projection
// the JSON endpoint returns.
func (cp *ControlPlane) statusPageHandler(c *gin.Context) {
	c.Data(http.StatusOK, "text/html; charset=utf-8", []byte(renderStatusHTML(cp.currentServiceStatus())))
}

// currentServiceStatus returns the projection used by the JSON endpoint and
// the HTML page. Separated so both can render from the same source.
func (cp *ControlPlane) currentServiceStatus() []serviceStatus {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	since := time.Now().Add(-5 * time.Minute).UTC().Format(time.RFC3339)
	resp, err := cp.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
		EventType: heartbeatEventType,
		TenantID:  heartbeatTenant,
		Since:     since,
		Limit:     500,
	})
	if err != nil || resp == nil {
		return nil
	}
	return projectStatus(resp.Events, time.Now())
}

// --------------- HTML status page ---------------

// renderStatusHTML renders a dead-simple HTML page. No JS framework, no
// client-side polling — the page is refreshed server-side on each hit.
// Keeps the dependency surface at zero beyond what CP already ships.
func renderStatusHTML(services []serviceStatus) string {
	overall := "healthy"
	for _, s := range services {
		if s.Status != "healthy" {
			overall = "degraded"
			break
		}
	}
	if len(services) == 0 {
		overall = "unknown"
	}

	var b strings.Builder
	b.WriteString(`<!doctype html><html><head><meta charset="utf-8"><title>AllSource Status</title>`)
	b.WriteString(`<meta http-equiv="refresh" content="15">`)
	b.WriteString(`<style>`)
	b.WriteString(`body{font-family:ui-sans-serif,system-ui,-apple-system,sans-serif;max-width:880px;margin:40px auto;padding:0 20px;color:#111;background:#fafafa}`)
	b.WriteString(`h1{font-size:28px;margin:0 0 8px}`)
	b.WriteString(`.overall{padding:20px;border-radius:12px;margin:16px 0 24px;font-weight:600}`)
	b.WriteString(`.overall.healthy{background:#dcfce7;color:#166534}`)
	b.WriteString(`.overall.degraded{background:#fee2e2;color:#991b1b}`)
	b.WriteString(`.overall.unknown{background:#fef3c7;color:#92400e}`)
	b.WriteString(`table{width:100%;border-collapse:collapse;background:#fff;border-radius:12px;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,.06)}`)
	b.WriteString(`th,td{padding:12px 16px;text-align:left;border-bottom:1px solid #eee}`)
	b.WriteString(`th{background:#f5f5f5;font-size:12px;text-transform:uppercase;letter-spacing:.5px;color:#666}`)
	b.WriteString(`.pill{display:inline-block;padding:2px 10px;border-radius:999px;font-size:12px;font-weight:600}`)
	b.WriteString(`.pill.healthy{background:#dcfce7;color:#166534}`)
	b.WriteString(`.pill.unhealthy{background:#fee2e2;color:#991b1b}`)
	b.WriteString(`.pill.stale{background:#fef3c7;color:#92400e}`)
	b.WriteString(`.pill.unknown{background:#e5e7eb;color:#374151}`)
	b.WriteString(`small{color:#666}`)
	b.WriteString(`</style></head><body>`)
	b.WriteString(`<h1>AllSource Status</h1>`)
	b.WriteString(`<small>Powered by event-sourced heartbeats — every probe is a permanent event in Core.</small>`)
	b.WriteString(`<div class="overall ` + overall + `">Overall: ` + overall + `</div>`)

	if len(services) == 0 {
		b.WriteString(`<p>No heartbeats received yet. The emitter may still be starting up.</p>`)
	} else {
		b.WriteString(`<table><thead><tr><th>Service</th><th>Status</th><th>Latency</th><th>Last seen</th><th>Detail</th></tr></thead><tbody>`)
		for _, s := range services {
			status := s.Status
			if status == "" {
				status = "unknown"
			}
			age := fmt.Sprintf("%.0fs ago", s.AgeSeconds)
			detail := s.Error
			if detail == "" {
				detail = s.ProbedURL
			}
			b.WriteString(`<tr>`)
			b.WriteString(`<td><strong>` + htmlEscape(s.Service) + `</strong></td>`)
			b.WriteString(`<td><span class="pill ` + htmlEscape(status) + `">` + htmlEscape(status) + `</span></td>`)
			b.WriteString(fmt.Sprintf(`<td>%dms</td>`, s.LatencyMs))
			b.WriteString(`<td><small>` + age + `</small></td>`)
			b.WriteString(`<td><small>` + htmlEscape(detail) + `</small></td>`)
			b.WriteString(`</tr>`)
		}
		b.WriteString(`</tbody></table>`)
	}

	b.WriteString(`<p><small>Auto-refreshes every 15s. JSON feed: <a href="/api/v1/status/services">/api/v1/status/services</a></small></p>`)
	b.WriteString(`</body></html>`)
	return b.String()
}

func htmlEscape(s string) string {
	r := strings.NewReplacer(
		"&", "&amp;",
		"<", "&lt;",
		">", "&gt;",
		"\"", "&quot;",
		"'", "&#39;",
	)
	return r.Replace(s)
}
