package usecases

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// communityTenantID is the tenant every request collapses onto when QS runs on
// ALLSOURCE_EDITION=community (the runbook §5 #5 trap). A non-community tenant
// with data is one whose id is something OTHER than this.
const communityTenantID = "community"

// systemTenantIDs are non-customer tenants excluded from the edition-trap
// data-tenant count (heartbeats, recovery audit, etc. live here).
var systemTenantIDs = map[string]bool{
	"system":         true,
	"admin-recovery": true,
}

// extractSubscriptionForHealth pulls SubscriptionMetadata (including the time
// fields the health model needs: SubscriptionEndsAt, TrialEndsAt, GrandfatherUntil)
// from a tenant's metadata map. Mirrors the extractSubscriptionEntity pattern in
// the billing package but parses the date fields too (which subscriptionFromMap
// there omits because dunning doesn't need them).
func extractSubscriptionForHealth(metadata map[string]interface{}) entities.SubscriptionMetadata {
	sub := entities.SubscriptionMetadata{}
	if metadata == nil {
		return sub
	}
	raw, ok := metadata["subscription"]
	if !ok {
		return sub
	}
	switch v := raw.(type) {
	case *entities.SubscriptionMetadata:
		return *v
	case entities.SubscriptionMetadata:
		return v
	case map[string]interface{}:
		return subscriptionMetaFromMap(v)
	default:
		return sub
	}
}

// subscriptionMetaFromMap decodes a JSON-ish map into SubscriptionMetadata,
// including the RFC3339 date fields used by the recency / expiry / grandfather
// signals.
func subscriptionMetaFromMap(m map[string]interface{}) entities.SubscriptionMetadata {
	s := entities.SubscriptionMetadata{}
	if v, ok := m["subscription_id"].(string); ok {
		s.SubscriptionID = v
	}
	if v, ok := m["customer_id"].(string); ok {
		s.CustomerID = v
	}
	if v, ok := m["status"].(string); ok {
		s.Status = v
	}
	if v, ok := m["tier"].(string); ok {
		s.Tier = v
	}
	if v, ok := m["billing_period"].(string); ok {
		s.BillingPeriod = v
	}
	s.TrialEndsAt = parseTimePtr(m["trial_ends_at"])
	s.SubscriptionEndsAt = parseTimePtr(m["subscription_ends_at"])
	s.GrandfatherUntil = parseTimePtr(m["grandfather_until"])
	return s
}

// parseTimePtr decodes a value that may be a time.Time, *time.Time, or an
// RFC3339 string into *time.Time. Returns nil for anything else / empty.
func parseTimePtr(v interface{}) *time.Time {
	switch t := v.(type) {
	case time.Time:
		if t.IsZero() {
			return nil
		}
		return &t
	case *time.Time:
		return t
	case string:
		if t == "" {
			return nil
		}
		if parsed, err := time.Parse(time.RFC3339Nano, t); err == nil {
			return &parsed
		}
		if parsed, err := time.Parse(time.RFC3339, t); err == nil {
			return &parsed
		}
	}
	return nil
}

// healthQuota is the minimal quota view the events_quota_pct signal needs.
type healthQuota struct {
	EventsUsed  int64
	EventsQuota int64
}

// extractQuotaForHealth pulls EventsUsed/EventsQuota from tenant metadata's
// "quotas" block (entities.QuotaMetadata json tags). Defaults to zeros.
func extractQuotaForHealth(metadata map[string]interface{}) healthQuota {
	q := healthQuota{}
	if metadata == nil {
		return q
	}
	raw, ok := metadata["quotas"]
	if !ok {
		return q
	}
	switch v := raw.(type) {
	case *entities.QuotaMetadata:
		q.EventsUsed, q.EventsQuota = v.EventsUsed, v.EventsQuota
	case entities.QuotaMetadata:
		q.EventsUsed, q.EventsQuota = v.EventsUsed, v.EventsQuota
	case map[string]interface{}:
		q.EventsUsed = toInt64Val(v["events_used"])
		q.EventsQuota = toInt64Val(v["events_quota"])
	}
	return q
}

// extractOverageEnabled reports whether overage billing is enabled for the
// tenant (the "overage" metadata block).
func extractOverageEnabled(metadata map[string]interface{}) bool {
	if metadata == nil {
		return false
	}
	raw, ok := metadata["overage"]
	if !ok {
		return false
	}
	switch v := raw.(type) {
	case *entities.OverageMetadata:
		return v.Enabled
	case entities.OverageMetadata:
		return v.Enabled
	case map[string]interface{}:
		// Decoding untyped metadata: a missing or non-bool "enabled" means
		// overage is not enabled, which is the correct conservative default.
		b, _ := v["enabled"].(bool) //nolint:forcetypeassert,errcheck // zero value is the intended default
		return b
	}
	return false
}

// effectiveBillingTier resolves the tenant's effective tier: the highest active
// tier among tracked subscriptions (bubble-up), falling back to the flat
// subscription.tier. Mirrors HighestActiveTier semantics.
func effectiveBillingTier(metadata map[string]interface{}, sub entities.SubscriptionMetadata) string {
	if metadata != nil {
		if raw, ok := metadata["subscriptions"]; ok {
			if subs := subscriptionsFromMap(raw); len(subs) > 0 {
				if tier, _ := entities.HighestActiveTier(subs); tier != "" {
					return tier
				}
			}
		}
	}
	if sub.Tier != "" {
		return sub.Tier
	}
	return string(entities.TierFree)
}

// subscriptionsFromMap decodes the tracked-subscriptions map from metadata.
func subscriptionsFromMap(raw interface{}) map[string]entities.SubscriptionRef {
	switch v := raw.(type) {
	case map[string]entities.SubscriptionRef:
		return v
	case map[string]interface{}:
		out := make(map[string]entities.SubscriptionRef, len(v))
		for id, item := range v {
			m, ok := item.(map[string]interface{})
			if !ok {
				continue
			}
			ref := entities.SubscriptionRef{}
			if s, ok := m["tier"].(string); ok {
				ref.Tier = s
			}
			if s, ok := m["status"].(string); ok {
				ref.Status = s
			}
			if s, ok := m["customer_id"].(string); ok {
				ref.CustomerID = s
			}
			out[id] = ref
		}
		return out
	default:
		return nil
	}
}

// durabilityFromHealth maps Core's /health response to (durable, warnings).
// Core returns {status, version, details}. A non-"healthy" status, or a
// details.durable==false, or a non-empty details.warnings list, is a durability
// concern.
func durabilityFromHealth(h *clients.HealthResponse) (bool, []string) {
	warnings := []string{}
	durable := true
	if h.Status != "" && h.Status != "healthy" && h.Status != "ok" {
		durable = false
		warnings = append(warnings, "core status="+h.Status)
	}
	if h.Details != nil {
		if v, ok := h.Details["durable"].(bool); ok && !v {
			durable = false
		}
		if ws, ok := h.Details["warnings"].([]interface{}); ok {
			for _, w := range ws {
				if s, ok := w.(string); ok && s != "" {
					warnings = append(warnings, s)
				}
			}
		}
	}
	return durable, warnings
}

// countNonCommunityDataTenants counts tenants that are NOT the community tenant
// and NOT a system tenant, and that have at least one event in Core. This is the
// fleet half of the edition-trap heuristic (§2.1).
func countNonCommunityDataTenants(ctx context.Context, uc *FleetHealthUseCase, tenants []*entities.Tenant) int {
	n := 0
	for _, t := range tenants {
		if t.ID == communityTenantID || systemTenantIDs[t.ID] {
			continue
		}
		stats := uc.tenantStats(ctx, t.ID)
		if stats != nil && stats.EventCount > 0 {
			n++
		}
	}
	return n
}

// httpGetJSON does a best-effort GET and decodes the body as a JSON object.
// Returns (nil, false) on any error or non-2xx.
func httpGetJSON(ctx context.Context, hc *http.Client, url string) (map[string]any, bool) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, http.NoBody)
	if err != nil {
		return nil, false
	}
	resp, err := hc.Do(req)
	if err != nil {
		return nil, false
	}
	defer func() { _ = resp.Body.Close() }() //nolint:errcheck // best-effort close
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		// QS returns 503 when degraded but the body still carries the edition;
		// accept up to 5xx-with-body so the edition probe survives a degraded QS.
		if resp.StatusCode < 200 || resp.StatusCode >= 600 {
			return nil, false
		}
	}
	body, err := io.ReadAll(io.LimitReader(resp.Body, 64*1024))
	if err != nil {
		return nil, false
	}
	var out map[string]any
	if err := json.Unmarshal(body, &out); err != nil {
		return nil, false
	}
	return out, true
}
