package signals

import (
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// Thresholds (from §2.1). Exported so tests and callers can reference the same
// boundaries the evaluators use.
const (
	LastEventDegradedAfter = 24 * time.Hour      // OK < 24h
	LastEventAtRiskAfter   = 7 * 24 * time.Hour  // At-Risk > 7d (paid tier)
	LastSyncDegradedAfter  = 1 * time.Hour       // OK < 1h
	LastSyncAtRiskAfter    = 24 * time.Hour      // At-Risk > 24h
	QuotaDegradedPct       = 80.0                // OK < 80%
	QuotaAtRiskPct         = 100.0               // At-Risk >= 100%
	SubExpiringWindow      = 7 * 24 * time.Hour  // At-Risk: renewal <= 7d
	GrandfatherExpirySoon  = 14 * 24 * time.Hour // flag: GrandfatherUntil <= 14d
	ReplicationDegradedMs  = 1000                // OK < 1s
	ReplicationAtRiskMs    = 10000               // At-Risk > 10s
)

// LastEventAge evaluates recency of the newest event for the tenant.
//
// newest is the most-recent event timestamp from the desc-probe (zero time =
// no events ever). hasData reports whether the tenant has any events at all.
// paidTier suppresses the At-Risk escalation for free tenants (a free tenant
// that never ingests is not "at risk" — it's just idle).
func LastEventAge(now, newest time.Time, hasData, paidTier bool) Signal {
	s := Signal{Name: SignalLastEventAge, Source: SourceCore}
	if !hasData || newest.IsZero() {
		s.Tier = TierHealthy
		s.Value = "never ingested"
		return s
	}
	age := now.Sub(newest)
	s.Value = humanAge(age)
	switch {
	case age > LastEventAtRiskAfter && paidTier:
		s.Tier = TierAtRisk
	case age > LastEventDegradedAfter:
		s.Tier = TierDegraded
	default:
		s.Tier = TierHealthy
	}
	return s
}

// LastSyncAge evaluates recency of the tenant's last *sync-source* event
// (prime.* / agent stream). newest is the desc-probe result filtered to the
// sync prefix; hasSync reports whether the tenant has a configured sync source
// (no sync source → n/a → Healthy, not a false alarm).
func LastSyncAge(now, newest time.Time, hasSync bool) Signal {
	s := Signal{Name: SignalLastSyncAge, Source: SourceCore}
	if !hasSync || newest.IsZero() {
		s.Tier = TierHealthy
		s.Value = "no sync source"
		return s
	}
	age := now.Sub(newest)
	s.Value = humanAge(age)
	switch {
	case age > LastSyncAtRiskAfter:
		s.Tier = TierAtRisk
	case age > LastSyncDegradedAfter:
		s.Tier = TierDegraded
	default:
		s.Tier = TierHealthy
	}
	return s
}

// EventsQuotaPct evaluates usage against quota. A -1 quota means unlimited → 0%.
// overageEnabled is informational here (the threshold doesn't change), but is
// recorded in the value so the operator sees whether being over-quota is billed
// or blocked.
func EventsQuotaPct(used, quota int64, overageEnabled bool) Signal {
	s := Signal{Name: SignalEventsQuotaPct, Source: SourceCoreMetadata}
	if quota < 0 {
		s.Tier = TierHealthy
		s.Value = "unlimited"
		return s
	}
	if quota == 0 {
		// No quota set yet (un-provisioned). Not an alarm; surface as 0%.
		s.Tier = TierHealthy
		s.Value = "0% (no quota)"
		return s
	}
	pct := float64(used) / float64(quota) * 100.0
	s.Value = pctString(pct)
	if overageEnabled {
		s.Value += " (overage on)"
	}
	switch {
	case pct >= QuotaAtRiskPct:
		s.Tier = TierAtRisk
	case pct >= QuotaDegradedPct:
		s.Tier = TierDegraded
	default:
		s.Tier = TierHealthy
	}
	return s
}

// SubscriptionState classifies the subscription. grandfatherActive is passed so
// the value can note grandfathering, but the Critical→suppression happens in
// Rollup (worst-wins override), not here — this evaluator reports the raw tier.
func SubscriptionState(sub entities.SubscriptionMetadata, now time.Time) Signal {
	s := Signal{Name: SignalSubscriptionState, Source: SourceCoreMetadata}
	status := normalizeStatus(sub.Status)
	s.Value = status
	if status == "" {
		// No subscription metadata at all — treat as free/none, Healthy.
		s.Value = "none"
		s.Tier = TierHealthy
		return s
	}

	switch status {
	case "active", "on_trial", "trialing":
		// Healthy unless the renewal/expiry is imminent.
		if end := effectiveEnd(sub); !end.IsZero() {
			until := end.Sub(now)
			if until > 0 && until <= SubExpiringWindow {
				s.Tier = TierAtRisk
				s.Value = status + " (expires " + humanAge(until) + ")"
				return s
			}
		}
		s.Tier = TierHealthy
	case "past_due":
		s.Tier = TierDegraded
		s.Value = "past_due (grace)"
	// Both spellings are matched on purpose: LemonSqueezy sends the British form.
	case "canceled", "cancelled", "unpaid", "expired": //nolint:misspell // mirrors LemonSqueezy status spelling
		s.Tier = TierCritical
	default:
		// Unknown status — be conservative but not critical.
		s.Tier = TierDegraded
	}
	return s
}

// GrandfatherWindow is informational: it never downgrades on its own. It flags
// "expiring soon" when GrandfatherUntil <= 14d, and reports active/none.
func GrandfatherWindow(sub entities.SubscriptionMetadata, now time.Time) Signal {
	s := Signal{Name: SignalGrandfatherWindow, Source: SourceCoreMetadata, Tier: TierHealthy}
	if sub.GrandfatherUntil == nil {
		s.Value = "none"
		return s
	}
	until := sub.GrandfatherUntil.Sub(now)
	if until <= 0 {
		s.Value = "expired"
		return s
	}
	if until <= GrandfatherExpirySoon {
		s.Value = "expiring soon (" + humanAge(until) + ")"
		// Still informational (Healthy tier) per §2.1 — surfaced as a reason via
		// the value, not by changing severity.
		return s
	}
	s.Value = "active (" + humanAge(until) + ")"
	return s
}

// EditionTrap. trapDetected is the cross-service determination from the QS probe
// + fleet heuristic (see EditionTrapDetected). The trap is fleet-wide, so the
// per-tenant signal is Critical for every tenant when the trap is present.
func EditionTrap(trapDetected bool) Signal {
	s := Signal{Name: SignalEditionTrap, Source: SourceQueryService, Tier: TierHealthy}
	if trapDetected {
		s.Tier = TierCritical
		s.Value = "QS edition=community pins every request to the community tenant"
	} else {
		s.Value = "ok"
	}
	return s
}

// Durability reads Core's deep-health view. durable=false or any non-empty
// warning is Critical. This is a fleet-wide Core property; like edition it maps
// onto every tenant.
func Durability(durable bool, warnings []string) Signal {
	s := Signal{Name: SignalDurability, Source: SourceCore, Tier: TierHealthy}
	if !durable {
		s.Tier = TierCritical
		s.Value = "core reports durable=false"
		return s
	}
	if len(warnings) > 0 {
		s.Tier = TierCritical
		s.Value = "core durability warnings: " + warnings[0]
		return s
	}
	s.Value = "durable"
	return s
}

// ReplicationLag from cluster status. maxLagMs is the worst follower lag;
// anyUnreachable flags a follower that can't be reached.
func ReplicationLag(maxLagMs int64, anyUnreachable bool) Signal {
	s := Signal{Name: SignalReplicationLag, Source: SourceCoreFollower, Tier: TierHealthy}
	if anyUnreachable {
		s.Tier = TierAtRisk
		s.Value = "follower unreachable"
		return s
	}
	s.Value = itoa(int(maxLagMs)) + "ms"
	switch {
	case maxLagMs > ReplicationAtRiskMs:
		s.Tier = TierAtRisk
	case maxLagMs > ReplicationDegradedMs:
		s.Tier = TierDegraded
	default:
		s.Tier = TierHealthy
	}
	return s
}

// APIKeyValidity asserts the tenant has a usable canonical key whose role string
// is exactly "serviceaccount" (no underscore — the service_account drift silently
// 403s every key, MEMORY.md). keyOK=false OR roleDrifted=true is Critical.
func APIKeyValidity(keyOK, roleDrifted bool) Signal {
	s := Signal{Name: SignalAPIKeyValidity, Source: SourceCore, Tier: TierHealthy}
	switch {
	case roleDrifted:
		s.Tier = TierCritical
		s.Value = "role string drifted (expected serviceaccount)"
	case !keyOK:
		s.Tier = TierCritical
		s.Value = "no valid API key"
	default:
		s.Value = "ok"
	}
	return s
}

// EmptyReadSymptom. symptomRate>0 for a tenant that HAS Core data points at a
// read-path/identity problem (never data loss — runbook lesson). hasCoreData
// gates the alarm: a genuinely-empty tenant returning 0 rows is correct.
func EmptyReadSymptom(symptomRate float64, hasCoreData bool) Signal {
	s := Signal{Name: SignalEmptyReadSymptom, Source: SourceQueryService, Tier: TierHealthy}
	if symptomRate > 0 && hasCoreData {
		s.Tier = TierDegraded
		s.Value = "reads return 0 rows / 404 despite Core data (read-path/identity)"
		return s
	}
	s.Value = "ok"
	return s
}

// --- small helpers ---

// effectiveEnd returns the soonest meaningful end time (subscription end, else
// trial end). Zero time when neither is set.
func effectiveEnd(sub entities.SubscriptionMetadata) time.Time {
	switch {
	case sub.SubscriptionEndsAt != nil:
		return *sub.SubscriptionEndsAt
	case sub.TrialEndsAt != nil:
		return *sub.TrialEndsAt
	default:
		return time.Time{}
	}
}

// pctString renders a percentage with one decimal (e.g. "92.5%").
func pctString(p float64) string {
	whole := int(p)
	frac := int((p - float64(whole)) * 10)
	if frac < 0 {
		frac = -frac
	}
	return itoa(whole) + "." + itoa(frac) + "%"
}

// PaidTier reports whether the effective tier is a paid tier (anything other
// than free / empty). Used to gate the last_event_age At-Risk escalation.
func PaidTier(tier string) bool {
	switch entities.SubscriptionTier(entities.MapRetiredTier(tier)) {
	case entities.TierIndie, entities.TierStudio, entities.TierScale, entities.TierEnterprise:
		return true
	default:
		return false
	}
}
