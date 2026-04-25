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
		//nolint:errcheck // test server: client hangup during Write is not actionable here
		w.Write([]byte(body))
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

func TestEmitterSnapshot_ReturnsLiveProbeResultsWithoutCore(t *testing.T) {
	// The whole point of the in-memory cache: snapshot must work even when
	// the Core client is nil / unreachable. This is the regression guard
	// for issue #160 (Core outage shouldn't take the status page down).
	h := newHeartbeatEmitter(nil, nil, nil)
	now := time.Date(2026, 4, 25, 18, 0, 0, 0, time.UTC)

	h.record("core", heartbeatSample{
		status:    "unhealthy",
		latencyMs: 5000,
		errMsg:    "dial tcp: lookup allsource-core.internal: no such host",
		at:        now.Add(-3 * time.Second),
	})
	h.record("control-plane", heartbeatSample{
		status:    "healthy",
		latencyMs: 4,
		at:        now.Add(-2 * time.Second),
	})

	out := h.snapshot(now)
	if len(out) != 2 {
		t.Fatalf("want 2 entries, got %d: %+v", len(out), out)
	}
	// Sorted by service name → control-plane first, then core.
	if out[0].Service != "control-plane" || out[0].Status != "healthy" {
		t.Errorf("control-plane: got %+v, want healthy", out[0])
	}
	if out[1].Service != "core" || out[1].Status != "unhealthy" {
		t.Errorf("core: got %+v, want unhealthy", out[1])
	}
	if out[1].Error == "" {
		t.Error("core: error message should be carried through to the projection")
	}
}

func TestEmitterSnapshot_FlipsToStaleAfterTTL(t *testing.T) {
	h := newHeartbeatEmitter(nil, nil, nil)
	now := time.Date(2026, 4, 25, 18, 0, 0, 0, time.UTC)

	// Recorded as healthy, but the sample is older than heartbeatTTL (25s).
	// snapshot must override "healthy" → "stale" so a stuck probe loop
	// can't masquerade as fresh.
	h.record("auth", heartbeatSample{
		status:    "healthy",
		latencyMs: 10,
		at:        now.Add(-45 * time.Second),
	})

	out := h.snapshot(now)
	if len(out) != 1 || out[0].Status != "stale" {
		t.Errorf("want stale entry for auth, got %+v", out)
	}
}
