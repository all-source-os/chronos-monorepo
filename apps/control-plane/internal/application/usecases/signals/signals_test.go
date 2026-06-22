package signals

import (
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
)

func TestRollup_WorstWins(t *testing.T) {
	tests := []struct {
		name        string
		sigs        []Signal
		grandfather bool
		want        Tier
	}{
		{
			name: "all healthy",
			sigs: []Signal{{Name: SignalDurability, Tier: TierHealthy}, {Name: SignalEventsQuotaPct, Tier: TierHealthy}},
			want: TierHealthy,
		},
		{
			name: "one degraded",
			sigs: []Signal{{Name: SignalEventsQuotaPct, Tier: TierDegraded}, {Name: SignalDurability, Tier: TierHealthy}},
			want: TierDegraded,
		},
		{
			name: "at-risk beats degraded",
			sigs: []Signal{{Name: SignalEventsQuotaPct, Tier: TierAtRisk}, {Name: SignalLastSyncAge, Tier: TierDegraded}},
			want: TierAtRisk,
		},
		{
			name: "critical beats all",
			sigs: []Signal{{Name: SignalEditionTrap, Tier: TierCritical}, {Name: SignalEventsQuotaPct, Tier: TierAtRisk}},
			want: TierCritical,
		},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if got := Rollup(tc.sigs, tc.grandfather); got != tc.want {
				t.Errorf("Rollup = %s, want %s", got, tc.want)
			}
		})
	}
}

func TestRollup_GrandfatherOverride(t *testing.T) {
	// subscription_state Critical + grandfather active → demoted to At-Risk.
	sigs := []Signal{
		{Name: SignalSubscriptionState, Tier: TierCritical},
		{Name: SignalEventsQuotaPct, Tier: TierDegraded},
	}
	if got := Rollup(sigs, true); got != TierAtRisk {
		t.Errorf("grandfathered subscription_critical → %s, want at_risk", got)
	}
	// Without grandfather, it stays Critical.
	if got := Rollup(sigs, false); got != TierCritical {
		t.Errorf("non-grandfathered subscription_critical → %s, want critical", got)
	}
	// Grandfather only suppresses subscription_state, NOT a different critical.
	sigs2 := []Signal{
		{Name: SignalSubscriptionState, Tier: TierCritical},
		{Name: SignalDurability, Tier: TierCritical},
	}
	if got := Rollup(sigs2, true); got != TierCritical {
		t.Errorf("grandfather must not suppress a durability critical: got %s, want critical", got)
	}
}

func TestEventsQuotaPct_Thresholds(t *testing.T) {
	tests := []struct {
		name        string
		used, quota int64
		want        Tier
	}{
		{"under 80%", 700_000, 1_000_000, TierHealthy},
		{"at 80% degraded", 800_000, 1_000_000, TierDegraded},
		{"at 100% at-risk", 1_000_000, 1_000_000, TierAtRisk},
		{"over quota at-risk", 1_200_000, 1_000_000, TierAtRisk},
		{"unlimited", 9_999_999, -1, TierHealthy},
		{"no quota set", 5, 0, TierHealthy},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			s := EventsQuotaPct(tc.used, tc.quota, false)
			if s.Tier != tc.want {
				t.Errorf("EventsQuotaPct(%d,%d) tier = %s, want %s (value=%s)", tc.used, tc.quota, s.Tier, tc.want, s.Value)
			}
			if s.Name != SignalEventsQuotaPct {
				t.Errorf("signal name = %s", s.Name)
			}
		})
	}
}

func TestLastEventAge_PaidGate(t *testing.T) {
	now := time.Now()
	old := now.Add(-10 * 24 * time.Hour) // 10 days

	// Paid tier, 10d stale → At-Risk.
	if s := LastEventAge(now, old, true, true); s.Tier != TierAtRisk {
		t.Errorf("paid 10d stale → %s, want at_risk", s.Tier)
	}
	// Free tier, 10d stale → only Degraded (the At-Risk escalation is gated).
	if s := LastEventAge(now, old, true, false); s.Tier != TierDegraded {
		t.Errorf("free 10d stale → %s, want degraded", s.Tier)
	}
	// Never ingested → Healthy (idle, not at risk).
	if s := LastEventAge(now, time.Time{}, false, true); s.Tier != TierHealthy {
		t.Errorf("never ingested → %s, want healthy", s.Tier)
	}
}

func TestSubscriptionState(t *testing.T) {
	now := time.Now()
	expSoon := now.Add(3 * 24 * time.Hour)
	tests := []struct {
		name string
		sub  entities.SubscriptionMetadata
		want Tier
	}{
		{"active healthy", entities.SubscriptionMetadata{Status: "active"}, TierHealthy},
		{"past_due degraded", entities.SubscriptionMetadata{Status: "past_due"}, TierDegraded},
		{"canceled critical", entities.SubscriptionMetadata{Status: "canceled"}, TierCritical},
		{"expired critical", entities.SubscriptionMetadata{Status: "expired"}, TierCritical},
		{"active expiring soon at-risk", entities.SubscriptionMetadata{Status: "active", SubscriptionEndsAt: &expSoon}, TierAtRisk},
		{"none healthy", entities.SubscriptionMetadata{}, TierHealthy},
	}
	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			if s := SubscriptionState(tc.sub, now); s.Tier != tc.want {
				t.Errorf("SubscriptionState(%s) = %s, want %s", tc.sub.Status, s.Tier, tc.want)
			}
		})
	}
}

func TestEditionTrap_And_Durability(t *testing.T) {
	if s := EditionTrap(true); s.Tier != TierCritical {
		t.Errorf("edition trap detected → %s, want critical", s.Tier)
	}
	if s := EditionTrap(false); s.Tier != TierHealthy {
		t.Errorf("no edition trap → %s, want healthy", s.Tier)
	}
	if s := Durability(false, nil); s.Tier != TierCritical {
		t.Errorf("durable=false → %s, want critical", s.Tier)
	}
	if s := Durability(true, []string{"wal lagging"}); s.Tier != TierCritical {
		t.Errorf("durability warning → %s, want critical", s.Tier)
	}
	if s := Durability(true, nil); s.Tier != TierHealthy {
		t.Errorf("durable=true no warnings → %s, want healthy", s.Tier)
	}
}

func TestReasons_SortedWorstFirst(t *testing.T) {
	a := Assessment{Signals: []Signal{
		{Name: SignalEventsQuotaPct, Tier: TierDegraded},
		{Name: SignalDurability, Tier: TierHealthy}, // excluded from reasons
		{Name: SignalEditionTrap, Tier: TierCritical},
		{Name: SignalLastSyncAge, Tier: TierAtRisk},
	}}
	reasons := a.Reasons()
	if len(reasons) != 3 {
		t.Fatalf("expected 3 reasons (healthy excluded), got %d", len(reasons))
	}
	if reasons[0].Tier != TierCritical || reasons[1].Tier != TierAtRisk || reasons[2].Tier != TierDegraded {
		t.Errorf("reasons not worst-first: %v", reasons)
	}
}

func TestAPIKeyValidity_RoleDrift(t *testing.T) {
	if s := APIKeyValidity(true, true); s.Tier != TierCritical {
		t.Errorf("role drift → %s, want critical", s.Tier)
	}
	if s := APIKeyValidity(false, false); s.Tier != TierCritical {
		t.Errorf("no valid key → %s, want critical", s.Tier)
	}
	if s := APIKeyValidity(true, false); s.Tier != TierHealthy {
		t.Errorf("valid key, no drift → %s, want healthy", s.Tier)
	}
}
