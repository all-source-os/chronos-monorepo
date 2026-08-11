package main

// GEO layer 4 — self-report capture on the API onboarding path.
//
// # Why the API path matters more than it looks
//
// `POST /api/v1/onboard/start` is published in apps/web/public/llms.txt and is
// how an agent signs up without ever touching the web UI. Those are exactly
// the AI-native users this whole measurement programme is about, and a capture
// that only covered the browser form would systematically miss them — while
// looking complete.
//
// # The vocabulary is a mirror, not an original
//
// The canonical discovery-source list is
// tooling/geo/geo-core/src/discovery.rs, serialised to
// docs/contracts/geo-events/discovery-sources.json. This file mirrors it, as
// does apps/web/src/lib/geo-discovery-sources.ts. The three cannot import each
// other (three languages, and the monorepo isolation rule), so each asserts
// against that one committed file in its own test suite —
// TestDiscoverySourcesMatchTheContract below.
//
// The failure that guards against is quiet: if this path wrote "ChatGPT" and
// the web form wrote "chatgpt", `geo report` would show two channels where
// there is one and the AI-sourced share — the headline number of the layer —
// would be silently halved.
//
// # Backward compatibility is non-negotiable
//
// Both fields are optional. An existing caller that omits them gets exactly
// the response it got before: same status, same keys, same values, and no
// event written. That endpoint has live agent callers in the wild.

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"strings"
	"time"
)

// geoDiscoverySource is one entry of the shared vocabulary.
type geoDiscoverySource struct {
	ID    string
	Label string
	AI    bool
}

// geoDiscoverySources mirrors docs/contracts/geo-events/discovery-sources.json.
// IDs are stable forever — renaming one splits a historical series in two with
// no way to stitch it back together.
var geoDiscoverySources = []geoDiscoverySource{
	{ID: "chatgpt", Label: "ChatGPT", AI: true},
	{ID: "claude", Label: "Claude", AI: true},
	{ID: "perplexity", Label: "Perplexity", AI: true},
	{ID: "gemini", Label: "Gemini", AI: true},
	{ID: "copilot", Label: "Microsoft Copilot", AI: true},
	{ID: "other-ai", Label: "Another AI assistant", AI: true},
	{ID: "search", Label: "Google or another search engine", AI: false},
	{ID: "x-twitter", Label: "X / Twitter", AI: false},
	{ID: "hn-reddit", Label: "Hacker News or Reddit", AI: false},
	{ID: "github", Label: "GitHub", AI: false},
	{ID: "word-of-mouth", Label: "Someone told me", AI: false},
	{ID: "other", Label: "Something else", AI: false},
}

// Capture paths — which signup path collected the answer. Stored in
// geo.selfreport.captured.source, and kept separate from the discovery source
// (what the human said sent them) because they answer different questions.
const (
	geoCapturePathWeb = "signup-form"
	geoCapturePathAPI = "onboard-api"
)

// Longest free-text prompt we will store. A real answer is a sentence; this
// bounds both abuse and how much user-submitted text we accumulate. Matches
// MAX_VERBATIM in apps/web/src/app/api/geo/self-report/route.ts.
const geoMaxVerbatim = 500

// geoDiscoverySourceIsKnown reports whether an id is in the vocabulary.
func geoDiscoverySourceIsKnown(id string) bool {
	for _, s := range geoDiscoverySources {
		if s.ID == id {
			return true
		}
	}
	return false
}

// geoDiscoverySourceIsAI reports whether an id counts toward the AI-sourced
// share. An unknown id is never AI.
func geoDiscoverySourceIsAI(id string) bool {
	for _, s := range geoDiscoverySources {
		if s.ID == id {
			return s.AI
		}
	}
	return false
}

// geoDeriveIdempotencyKey hashes a natural key exactly as
// geo_core::idempotency::derive_key does: SHA-256 over the parts joined by a
// Unit Separator byte (0x1f), first 16 bytes as hex.
//
// Byte-compatibility with the Rust and TypeScript producers is not cosmetic.
// All three write geo.selfreport.captured, and if their keys disagreed the
// same capture would land in Core as two entities and every layer-4 count
// would silently inflate.
func geoDeriveIdempotencyKey(parts ...string) string {
	h := sha256.New()
	for i, part := range parts {
		if i > 0 {
			h.Write([]byte{0x1f})
		}
		h.Write([]byte(part))
	}
	return hex.EncodeToString(h.Sum(nil)[:16])
}

// geoSelfReportEnvelope is the Core ingest body for one self-report.
//
// Core assigns id/timestamp/version; the gateway injects tenant_id. Neither is
// ours to send. Note this is the Control Plane writing THROUGH itself to Core,
// so the tenant_id is set explicitly here — the same way buildSampleEvents
// does on this path.
type geoSelfReportEnvelope struct {
	EventType string                 `json:"event_type"`
	EntityID  string                 `json:"entity_id"`
	TenantID  string                 `json:"tenant_id"`
	Payload   map[string]interface{} `json:"payload"`
	Metadata  map[string]interface{} `json:"metadata"`
}

// buildGeoSelfReport builds the geo.selfreport.captured envelope for an
// onboarding capture, or returns ok=false when there is nothing to record.
//
// Returning ok=false rather than an error is deliberate: an absent or
// unrecognised discovery_source must never fail a signup. The endpoint's job
// is to mint a tenant; attribution is a bonus and is silently skipped when the
// caller did not give us a usable answer.
func buildGeoSelfReport(
	tenantID, tier, source, prompt string,
	observedAt time.Time,
) (geoSelfReportEnvelope, bool) {
	source = strings.TrimSpace(source)
	if !geoDiscoverySourceIsKnown(source) {
		return geoSelfReportEnvelope{}, false
	}

	// Second resolution, UTC, RFC 3339 — the natural key is derived from the
	// same string, so sub-second jitter cannot split one capture into two.
	// Rendered exactly as chrono does, which is what makes the key match.
	observed := observedAt.UTC().Format("2006-01-02T15:04:05Z")

	key := geoDeriveIdempotencyKey(observed, geoCapturePathAPI, source, tenantID)

	// The free text is only meaningful for the AI options — "what did you ask
	// it?" is nonsense for someone who arrived from GitHub, and storing an
	// answer to a question we did not ask would make the field a
	// general-purpose comment box we then have to hold.
	var verbatim interface{}
	if geoDiscoverySourceIsAI(source) {
		trimmed := strings.TrimSpace(prompt)
		if len(trimmed) > geoMaxVerbatim {
			trimmed = trimmed[:geoMaxVerbatim]
		}
		if trimmed != "" {
			verbatim = trimmed
		}
	}

	var tierValue interface{}
	if tier != "" {
		tierValue = tier
	}

	return geoSelfReportEnvelope{
		EventType: "geo.selfreport.captured",
		EntityID:  fmt.Sprintf("geo:selfreport:%s", key),
		TenantID:  tenantID,
		Payload: map[string]interface{}{
			"schema_version": 1,
			"observed_at":    observed,
			// Which path captured it, and what they said sent them.
			"source":   geoCapturePathAPI,
			"surface":  source,
			"verbatim": verbatim,
			// contact_ref is the tenant id — NEVER the email address. GEO
			// telemetry is a trend timeline, not a place to accumulate PII.
			"contact_ref": tenantID,
			"tier":        tierValue,
		},
		Metadata: map[string]interface{}{
			"emitter":         "tooling/geo",
			"idempotency_key": key,
		},
	}, true
}
