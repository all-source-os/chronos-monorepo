package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// contractDir is the repository's geo event contract, reached from this
// package. Tests run from a full checkout; the Docker build context for this
// app is apps/control-plane and does NOT include docs/, which is why the two
// contract tests below skip rather than fail when the file is absent. Skipping
// is safe because the file only ever disappears in a context where these tests
// are not the gate — a sparse checkout — while drift in a real checkout still
// fails loudly.
func contractFile(t *testing.T, name string) []byte {
	t.Helper()
	path := filepath.Join("..", "..", "docs", "contracts", "geo-events", name)
	data, err := os.ReadFile(path) //nolint:gosec // fixed repo-relative path
	if err != nil {
		t.Skipf("contract file %s not available in this checkout: %v", path, err)
	}
	return data
}

// The discovery-source vocabulary must match the generated contract exactly.
//
// Three sides speak it — this file, apps/web/src/lib/geo-discovery-sources.ts
// and tooling/geo/geo-core — and none can import the others. If they drifted,
// `geo report` would show two channels where there is one and the AI-sourced
// share would be silently halved.
func TestDiscoverySourcesMatchTheContract(t *testing.T) {
	var contract struct {
		CapturePaths []string `json:"capture_paths"`
		Sources      []struct {
			ID    string `json:"id"`
			Label string `json:"label"`
			AI    bool   `json:"ai"`
		} `json:"sources"`
	}
	if err := json.Unmarshal(contractFile(t, "discovery-sources.json"), &contract); err != nil {
		t.Fatalf("contract is not readable JSON: %v", err)
	}

	if len(contract.Sources) != len(geoDiscoverySources) {
		t.Fatalf("contract has %d sources, this file has %d — regenerate and mirror it",
			len(contract.Sources), len(geoDiscoverySources))
	}
	for i, want := range contract.Sources {
		got := geoDiscoverySources[i]
		if got.ID != want.ID || got.Label != want.Label || got.AI != want.AI {
			t.Errorf("source %d: have {%q %q %v}, contract says {%q %q %v}",
				i, got.ID, got.Label, got.AI, want.ID, want.Label, want.AI)
		}
	}

	wantPaths := []string{geoCapturePathWeb, geoCapturePathAPI}
	if len(contract.CapturePaths) != len(wantPaths) {
		t.Fatalf("contract has %d capture paths, this file has %d",
			len(contract.CapturePaths), len(wantPaths))
	}
	for i, want := range contract.CapturePaths {
		if wantPaths[i] != want {
			t.Errorf("capture path %d: have %q, contract says %q", i, wantPaths[i], want)
		}
	}
}

// The envelope this file builds must be byte-compatible with the one geo-core
// generates — same entity_id, same idempotency key — or one capture would land
// in Core as two entities.
func TestSelfReportEnvelopeMatchesTheGeneratedExample(t *testing.T) {
	var example struct {
		EntityID string `json:"entity_id"`
		Payload  struct {
			ObservedAt string `json:"observed_at"`
			Source     string `json:"source"`
			Surface    string `json:"surface"`
			ContactRef string `json:"contact_ref"`
		} `json:"payload"`
		Metadata struct {
			IdempotencyKey string `json:"idempotency_key"`
		} `json:"metadata"`
	}
	raw := contractFile(t, filepath.Join("examples", "geo.selfreport.captured.json"))
	if err := json.Unmarshal(raw, &example); err != nil {
		t.Fatalf("example is not readable JSON: %v", err)
	}

	// The example was written on the web capture path; derive the same key
	// from the same natural-key tuple and check the hash agrees.
	key := geoDeriveIdempotencyKey(
		example.Payload.ObservedAt,
		example.Payload.Source,
		example.Payload.Surface,
		example.Payload.ContactRef,
	)
	if key != example.Metadata.IdempotencyKey {
		t.Errorf("derived key %s != geo-core's %s — the Go and Rust hashes have drifted",
			key, example.Metadata.IdempotencyKey)
	}
	if want := "geo:selfreport:" + key; want != example.EntityID {
		t.Errorf("entity_id %s != %s", want, example.EntityID)
	}
}

func TestBuildGeoSelfReport(t *testing.T) {
	at := time.Date(2026, 8, 11, 11, 30, 0, 987_000_000, time.UTC)

	t.Run("an absent source records nothing", func(t *testing.T) {
		if _, ok := buildGeoSelfReport("t1", "trial", "", "", at); ok {
			t.Error("an omitted discovery_source must not produce an event")
		}
	})

	t.Run("an unrecognised source records nothing rather than failing", func(t *testing.T) {
		// Never an error: the endpoint's job is to mint a tenant. A typo in an
		// optional telemetry field must not cost somebody their signup.
		for _, bad := range []string{"ChatGPT", "chatgpt-6", "  ", "'; DROP TABLE"} {
			if _, ok := buildGeoSelfReport("t1", "trial", bad, "", at); ok {
				t.Errorf("%q should not have produced an event", bad)
			}
		}
	})

	t.Run("a known source produces a well-formed envelope", func(t *testing.T) {
		env, ok := buildGeoSelfReport("t1", "trial", "chatgpt", "agent memory store?", at)
		if !ok {
			t.Fatal("chatgpt is a known source")
		}
		if env.EventType != "geo.selfreport.captured" {
			t.Errorf("event_type = %q", env.EventType)
		}
		// The event lands in OUR telemetry tenant, never the customer's —
		// otherwise our operational data sits in their stream and the geo.*
		// family scatters across every tenant that ever signed up.
		if env.TenantID != defaultGeoTenant {
			t.Errorf("tenant_id = %q, want the GEO telemetry tenant %q", env.TenantID, defaultGeoTenant)
		}
		if env.TenantID == "t1" {
			t.Error("telemetry was written into the signing-up tenant's own event stream")
		}
		// Truncated to whole seconds so sub-second jitter cannot split one
		// capture into two entities.
		if got := env.Payload["observed_at"]; got != "2026-08-11T11:30:00Z" {
			t.Errorf("observed_at = %v, want whole seconds", got)
		}
		if got := env.Payload["source"]; got != geoCapturePathAPI {
			t.Errorf("source = %v, want the API capture path", got)
		}
		if got := env.Payload["surface"]; got != "chatgpt" {
			t.Errorf("surface = %v", got)
		}
		if got := env.Payload["verbatim"]; got != "agent memory store?" {
			t.Errorf("verbatim = %v", got)
		}
		// contact_ref is the tenant id, never an email address.
		if got := env.Payload["contact_ref"]; got != "t1" {
			t.Errorf("contact_ref = %v", got)
		}
		if got := env.Payload["tier"]; got != "trial" {
			t.Errorf("tier = %v", got)
		}
	})

	t.Run("free text is dropped for non-AI sources", func(t *testing.T) {
		env, ok := buildGeoSelfReport("t1", "trial", "github", "what did you ask it", at)
		if !ok {
			t.Fatal("github is a known source")
		}
		if got := env.Payload["verbatim"]; got != nil {
			t.Errorf("verbatim = %v, want nil — we never asked this question", got)
		}
	})

	t.Run("free text is bounded", func(t *testing.T) {
		long := make([]byte, geoMaxVerbatim+200)
		for i := range long {
			long[i] = 'a'
		}
		env, _ := buildGeoSelfReport("t1", "trial", "claude", string(long), at)
		got, _ := env.Payload["verbatim"].(string)
		if len(got) != geoMaxVerbatim {
			t.Errorf("verbatim length = %d, want %d", len(got), geoMaxVerbatim)
		}
	})

	t.Run("the telemetry tenant is overridable", func(t *testing.T) {
		// It must match the tenant the ALLSOURCE_API_KEY used by geo report
		// and apps/web belongs to, or the web and API captures land in two
		// tenants and the report shows one of them.
		t.Setenv("GEO_TENANT_ID", "geo-telemetry")
		env, _ := buildGeoSelfReport("t1", "trial", "chatgpt", "", at)
		if env.TenantID != "geo-telemetry" {
			t.Errorf("tenant_id = %q, want the override", env.TenantID)
		}
	})

	t.Run("an empty tier is null, not an empty string", func(t *testing.T) {
		env, _ := buildGeoSelfReport("t1", "", "claude", "", at)
		if got := env.Payload["tier"]; got != nil {
			t.Errorf("tier = %v, want nil", got)
		}
	})

	t.Run("the same capture twice is one entity", func(t *testing.T) {
		a, _ := buildGeoSelfReport("t1", "trial", "chatgpt", "", at)
		b, _ := buildGeoSelfReport("t1", "indie", "chatgpt", "later free text", at)
		if a.EntityID != b.EntityID {
			t.Errorf("a restate minted a second entity: %s vs %s", a.EntityID, b.EntityID)
		}
		// ...and two different tenants are two entities.
		c, _ := buildGeoSelfReport("t2", "trial", "chatgpt", "", at)
		if a.EntityID == c.EntityID {
			t.Error("two tenants collapsed into one entity")
		}
	})
}

// Backward compatibility, pinned. This endpoint is published in
// apps/web/public/llms.txt and live agents call it: a body with only
// email/name must still bind, and must leave the new fields empty so no event
// is written and the response is byte-for-byte what it always was.
func TestOnboardRequestBindsWithoutTheGeoFields(t *testing.T) {
	var req OnboardRequest
	body := []byte(`{"email":"you@example.com","name":"GEO measurement"}`)
	if err := json.Unmarshal(body, &req); err != nil {
		t.Fatalf("the pre-existing request shape no longer binds: %v", err)
	}
	if req.Email != "you@example.com" || req.Name != "GEO measurement" {
		t.Fatalf("existing fields changed meaning: %+v", req)
	}
	if req.DiscoverySource != "" || req.DiscoveryPrompt != "" {
		t.Fatalf("new fields are not empty by default: %+v", req)
	}
	if _, ok := buildGeoSelfReport("t1", "trial", req.DiscoverySource, req.DiscoveryPrompt, time.Now()); ok {
		t.Error("an old-shape caller must not produce a self-report event")
	}
}
