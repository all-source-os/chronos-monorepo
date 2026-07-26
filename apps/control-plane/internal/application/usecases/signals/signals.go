// Package signals computes the per-tenant health signals defined in
// docs/proposals/CONTROL_PLANE_TENANT_HEALTH_RECOVERY.md §2.1 and folds them
// into a single tier via the §2.2 worst-wins rollup (with the grandfather
// override).
//
// Each signal names its source backend and a threshold. A signal evaluation
// produces a Signal{Name, Value, Tier, Source}; the rollup converts the set of
// signals into one Tier and carries the contributing-signal list so a consumer
// can show *why* a tenant is non-green, not just the color.
//
// Architecture note (CLAUDE.md / MEMORY.md): Core IS the database. A tenant that
// "reports empty data" is classified here as a read-path / identity symptom
// (empty_read_symptom_rate), NEVER as data loss. Recency comes from a documented
// `?limit=1&order=desc` probe, not a stored last_event_at column.
package signals

import (
	"strings"
	"time"
)

// Tier is the four-level health classification (worst → best, by Severity).
type Tier string

// Tier constants.
const (
	TierHealthy  Tier = "healthy"
	TierDegraded Tier = "degraded"
	TierAtRisk   Tier = "at_risk"
	TierCritical Tier = "critical"
)

// Severity orders tiers for worst-wins. Higher = worse.
func (t Tier) Severity() int {
	switch t {
	case TierCritical:
		return 3
	case TierAtRisk:
		return 2
	case TierDegraded:
		return 1
	case TierHealthy:
		return 0
	default:
		return 0
	}
}

// Signal names (stable strings — consumed by the admin UI and MCP).
const (
	SignalLastEventAge      = "last_event_age"
	SignalLastSyncAge       = "last_sync_age"
	SignalEventsQuotaPct    = "events_quota_pct"
	SignalSubscriptionState = "subscription_state"
	SignalGrandfatherWindow = "grandfather_window"
	SignalEditionTrap       = "edition_trap"
	SignalDurability        = "durability"
	SignalReplicationLag    = "replication_lag"
	SignalAPIKeyValidity    = "api_key_validity"
	SignalEmptyReadSymptom  = "empty_read_symptom_rate"
)

// Source backends a signal can be read from.
const (
	SourceCore         = "core"
	SourceCoreMetadata = "core_metadata"
	SourceQueryService = "query_service"
	SourceCoreFollower = "core_followers"
)

// Signal is one evaluated health signal for a tenant.
type Signal struct {
	Name   string `json:"signal"`
	Value  string `json:"value"`
	Tier   Tier   `json:"tier"`
	Source string `json:"source,omitempty"`
}

// Assessment is a tenant's full health result: the rolled-up tier plus the
// ordered list of contributing signals (worst-first).
type Assessment struct {
	TenantID string   `json:"tenant_id"`
	Name     string   `json:"name,omitempty"`
	EffTier  string   `json:"effective_tier,omitempty"` // billing tier (free/indie/…)
	Tier     Tier     `json:"tier"`                     // health tier
	Signals  []Signal `json:"signals"`
}

// Reasons returns only the signals that pushed the tenant off Healthy, ordered
// worst-first. This is the `reasons` list the fleet rollup surfaces.
func (a Assessment) Reasons() []Signal {
	out := make([]Signal, 0, len(a.Signals))
	for _, s := range a.Signals {
		if s.Tier != TierHealthy && s.Tier != "" {
			out = append(out, s)
		}
	}
	sortBySeverityDesc(out)
	return out
}

// Rollup applies the §2.2 worst-wins rule with the grandfather override:
//
//	tier = Critical  if ANY signal is Critical
//	                 AND NOT (the only Critical signal is subscription_state
//	                          AND grandfather_window is active)
//	       At-Risk   else if ANY signal is At-Risk
//	       Degraded  else if ANY signal is Degraded
//	       Healthy   else
//
// grandfatherActive is the IsGrandfathered(now) result for the tenant; it
// suppresses a subscription_state=Critical downgrade (and ONLY that one) while
// the window is open.
func Rollup(sigs []Signal, grandfatherActive bool) Tier {
	worst := TierHealthy
	criticalCount := 0
	onlyCriticalIsSubscription := true

	for _, s := range sigs {
		if s.Tier.Severity() > worst.Severity() {
			worst = s.Tier
		}
		if s.Tier == TierCritical {
			criticalCount++
			if s.Name != SignalSubscriptionState {
				onlyCriticalIsSubscription = false
			}
		}
	}

	// Grandfather override: if the ONLY Critical signal is subscription_state
	// and the tenant is inside its grandfather window, demote to At-Risk (the
	// next-worst observed tier still applies for the other signals).
	if worst == TierCritical && grandfatherActive && criticalCount > 0 && onlyCriticalIsSubscription {
		// Recompute worst excluding subscription_state criticals.
		demoted := TierHealthy
		for _, s := range sigs {
			t := s.Tier
			if s.Name == SignalSubscriptionState && t == TierCritical {
				// grandfathered: treat the critical subscription as At-Risk
				t = TierAtRisk
			}
			if t.Severity() > demoted.Severity() {
				demoted = t
			}
		}
		return demoted
	}

	return worst
}

// sortBySeverityDesc sorts signals worst-first (stable insertion-ish — small N).
func sortBySeverityDesc(sigs []Signal) {
	for i := 1; i < len(sigs); i++ {
		for j := i; j > 0 && sigs[j].Tier.Severity() > sigs[j-1].Tier.Severity(); j-- {
			sigs[j], sigs[j-1] = sigs[j-1], sigs[j]
		}
	}
}

// --- shared time helpers used by recency-style signals ---

// ParseEventTime parses a Core event timestamp, tolerating both RFC3339Nano and
// RFC3339. Returns zero time + false if unparseable. Exported for callers that
// drive the recency desc-probe.
func ParseEventTime(ts string) (time.Time, bool) {
	if ts == "" {
		return time.Time{}, false
	}
	if t, err := time.Parse(time.RFC3339Nano, ts); err == nil {
		return t, true
	}
	if t, err := time.Parse(time.RFC3339, ts); err == nil {
		return t, true
	}
	return time.Time{}, false
}

// humanAge renders a duration compactly for a signal value (e.g. "3h", "5d").
func humanAge(d time.Duration) string {
	switch {
	case d < time.Minute:
		return "just now"
	case d < time.Hour:
		return itoa(int(d.Minutes())) + "m"
	case d < 24*time.Hour:
		return itoa(int(d.Hours())) + "h"
	default:
		return itoa(int(d.Hours()/24)) + "d"
	}
}

// itoa is a tiny strconv.Itoa to avoid importing strconv across the package for
// one call site; kept private.
func itoa(n int) string {
	if n == 0 {
		return "0"
	}
	neg := n < 0
	if neg {
		n = -n
	}
	var b [20]byte
	i := len(b)
	for n > 0 {
		i--
		b[i] = byte('0' + n%10)
		n /= 10
	}
	if neg {
		i--
		b[i] = '-'
	}
	return string(b[i:])
}

// normalizeStatus lowercases and trims a subscription status for comparison.
func normalizeStatus(s string) string {
	return strings.ToLower(strings.TrimSpace(s))
}
