package main

import (
	"context"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

func newTestServer(status int, body string) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(status)
		_, _ = w.Write([]byte(body))
	}))
}

func testContext() context.Context { return context.Background() }

func TestProjectStatus_PicksNewestPerService(t *testing.T) {
	now := time.Date(2026, 4, 17, 20, 0, 0, 0, time.UTC)
	old := now.Add(-45 * time.Second)  // > heartbeatTTL (25s) → stale
	fresh := now.Add(-5 * time.Second) // < heartbeatTTL → honors reported status

	events := []clients.EventEntry{
		// Two heartbeats for "core": an old-healthy and a fresher-unhealthy.
		// Projection should pick the newer one.
		{
			EventType: heartbeatEventType,
			EntityID:  "core",
			Timestamp: old.Format(time.RFC3339Nano),
			Payload: map[string]any{
				"status":     "healthy",
				"latency_ms": float64(12),
			},
		},
		{
			EventType: heartbeatEventType,
			EntityID:  "core",
			Timestamp: fresh.Format(time.RFC3339Nano),
			Payload: map[string]any{
				"status":     "unhealthy",
				"latency_ms": float64(5000),
				"error":      "http 502",
			},
		},
		// One service whose only heartbeat is old — should flip to "stale"
		// regardless of what the payload says.
		{
			EventType: heartbeatEventType,
			EntityID:  "query",
			Timestamp: old.Format(time.RFC3339Nano),
			Payload:   map[string]any{"status": "healthy"},
		},
		// Noise: a non-heartbeat event in the same query window should be
		// ignored, not treated as a service.
		{
			EventType: "user.signed_up",
			EntityID:  "alice",
			Timestamp: fresh.Format(time.RFC3339Nano),
		},
	}

	out := projectStatus(events, now)
	if len(out) != 2 {
		t.Fatalf("want 2 services, got %d: %+v", len(out), out)
	}

	// Output is sorted by service name.
	if out[0].Service != "core" {
		t.Errorf("out[0]: got %q, want core", out[0].Service)
	}
	if out[0].Status != "unhealthy" {
		t.Errorf("core status: got %q, want unhealthy (newer heartbeat wins)", out[0].Status)
	}
	if out[0].Error != "http 502" {
		t.Errorf("core error: got %q, want http 502", out[0].Error)
	}
	if out[0].LatencyMs != 5000 {
		t.Errorf("core latency: got %d, want 5000", out[0].LatencyMs)
	}

	if out[1].Service != "query" {
		t.Errorf("out[1]: got %q, want query", out[1].Service)
	}
	if out[1].Status != "stale" {
		t.Errorf("query status: got %q, want stale (heartbeat older than TTL)", out[1].Status)
	}
}

func TestProjectStatus_EmptyInput(t *testing.T) {
	out := projectStatus(nil, time.Now())
	if len(out) != 0 {
		t.Errorf("want empty slice, got %d entries", len(out))
	}
}

func TestProbeOnce_StatusAndBodyMatch(t *testing.T) {
	// Spin up a test server that returns different shapes depending on path.
	t.Run("200 with body match → healthy", func(t *testing.T) {
		h := newHeartbeatEmitter(nil, nil, nil)
		srv := newTestServer(200, `{"status":"healthy"}`)
		defer srv.Close()

		status, err := h.probeOnce(testContext(), probe{
			url:       srv.URL,
			bodyMatch: `"status":"healthy"`,
		})
		if status != "healthy" || err != "" {
			t.Errorf("got (%q, %q), want (healthy, )", status, err)
		}
	})

	t.Run("200 but body missing marker → unhealthy", func(t *testing.T) {
		h := newHeartbeatEmitter(nil, nil, nil)
		srv := newTestServer(200, `{"status":"degraded"}`)
		defer srv.Close()

		status, msg := h.probeOnce(testContext(), probe{
			url:       srv.URL,
			bodyMatch: `"status":"healthy"`,
		})
		if status != "unhealthy" {
			t.Errorf("status: got %q, want unhealthy", status)
		}
		if !strings.Contains(msg, "missing expected marker") {
			t.Errorf("error: got %q, want 'missing expected marker'", msg)
		}
	})

	t.Run("500 → unhealthy with http status in error", func(t *testing.T) {
		h := newHeartbeatEmitter(nil, nil, nil)
		srv := newTestServer(500, `internal server error`)
		defer srv.Close()

		status, msg := h.probeOnce(testContext(), probe{url: srv.URL})
		if status != "unhealthy" {
			t.Errorf("status: got %q, want unhealthy", status)
		}
		if !strings.Contains(msg, "http 500") {
			t.Errorf("error: got %q, want 'http 500'", msg)
		}
	})
}

func TestRenderStatusHTML_ShowsOverallAndPerService(t *testing.T) {
	services := []serviceStatus{
		{Service: "core", Status: "healthy", LatencyMs: 12, LastSeen: "2026-04-17T20:00:00Z", AgeSeconds: 3},
		{Service: "prime", Status: "unhealthy", LatencyMs: 0, LastSeen: "2026-04-17T19:59:30Z", AgeSeconds: 33, Error: "connection refused"},
	}
	html := renderStatusHTML(services)

	for _, want := range []string{
		`Overall: degraded`, // at least one unhealthy → degraded banner
		`<strong>core</strong>`,
		`<strong>prime</strong>`,
		`pill healthy">healthy`,
		`pill unhealthy">unhealthy`,
		`connection refused`,
	} {
		if !strings.Contains(html, want) {
			t.Errorf("HTML missing %q", want)
		}
	}
}

func TestRenderStatusHTML_OverallHealthyWhenAllHealthy(t *testing.T) {
	services := []serviceStatus{
		{Service: "a", Status: "healthy"},
		{Service: "b", Status: "healthy"},
	}
	html := renderStatusHTML(services)
	if !strings.Contains(html, `Overall: healthy`) {
		t.Error("want Overall: healthy banner when all services healthy")
	}
}

func TestRenderStatusHTML_UnknownWhenEmpty(t *testing.T) {
	html := renderStatusHTML(nil)
	if !strings.Contains(html, `Overall: unknown`) {
		t.Error("want Overall: unknown when no services reported yet")
	}
}
