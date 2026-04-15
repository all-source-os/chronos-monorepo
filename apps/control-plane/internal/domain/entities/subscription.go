package entities

import "time"

// SubscriptionTier represents a billing tier.
type SubscriptionTier string

// Subscription tier constants matching LemonSqueezy products. Canonical tier
// ladder per docs/marketing/PRICING_DECISION_2026-04.md: free → pro → growth →
// enterprise. TierTeam is a legacy alias retained so old webhook payloads and
// Core tenant metadata don't break; new code should use TierGrowth.
const (
	TierFree       SubscriptionTier = "free"       // $0, 100K events/mo
	TierPro        SubscriptionTier = "pro"        // $29/mo, 1M events/mo (x402 unlocked)
	TierGrowth     SubscriptionTier = "growth"     // $79/mo (annual) or $99/mo, 10M events/mo
	TierEnterprise SubscriptionTier = "enterprise" // Custom, unlimited
	TierTeam       SubscriptionTier = "team"       // Deprecated: legacy alias for growth
)

// SubscriptionMetadata holds billing/subscription data stored in Core tenant metadata.
type SubscriptionMetadata struct {
	CustomerID         string     `json:"customer_id,omitempty"`
	SubscriptionID     string     `json:"subscription_id,omitempty"`
	SubscriptionItemID string     `json:"subscription_item_id,omitempty"` // LemonSqueezy subscription item for metered billing
	Status             string     `json:"status,omitempty"`               // active, past_due, canceled, trialing, expired
	Tier               string     `json:"tier"`
	PaymentProvider    string     `json:"payment_provider,omitempty"` // "lemonsqueezy" or "stripe"
	TrialEndsAt        *time.Time `json:"trial_ends_at,omitempty"`
	SubscriptionEndsAt *time.Time `json:"subscription_ends_at,omitempty"`
}

// QuotaMetadata holds usage quota data stored in Core tenant metadata.
type QuotaMetadata struct {
	EventsQuota  int64  `json:"events_quota"`
	QueriesQuota int64  `json:"queries_quota"`
	EventsUsed   int64  `json:"events_used"`
	QueriesUsed  int64  `json:"queries_used"`
	ResetDate    string `json:"reset_date,omitempty"` // ISO-8601 date for next reset
}

// OverageMetadata holds overage billing data stored in Core tenant metadata.
type OverageMetadata struct {
	Enabled             bool    `json:"enabled"`
	EventRate           float64 `json:"event_rate,omitempty"` // cost per event over quota
	QueryRate           float64 `json:"query_rate,omitempty"` // cost per query over quota
	EventsOverage       int64   `json:"events_overage"`
	QueriesOverage      int64   `json:"queries_overage"`
	LastReportedEvents  int64   `json:"last_reported_events,omitempty"`  // last reported overage to prevent double-reporting
	LastReportedQueries int64   `json:"last_reported_queries,omitempty"` // last reported overage to prevent double-reporting
}

// TenantBillingMetadata is the top-level structure for all billing-related
// metadata stored in Core's tenant metadata map.
type TenantBillingMetadata struct {
	Subscription *SubscriptionMetadata `json:"subscription,omitempty"`
	Quotas       *QuotaMetadata        `json:"quotas,omitempty"`
	Overage      *OverageMetadata      `json:"overage,omitempty"`
}

// TierQuotas defines the quota limits for a given tier.
type TierQuotas struct {
	EventsQuota  int64
	QueriesQuota int64
}

// TierQuotaMap maps tiers to their quota limits. Numbers come from the April
// 2026 pricing decision memo (docs/marketing/PRICING_DECISION_2026-04.md).
// TierEnterprise uses -1 to signal unlimited (see TierQuotas.IsUnlimited).
// TierTeam mirrors TierGrowth for backwards compatibility with old payloads.
var TierQuotaMap = map[SubscriptionTier]TierQuotas{
	TierFree:       {EventsQuota: 100_000, QueriesQuota: 10_000},
	TierPro:        {EventsQuota: 1_000_000, QueriesQuota: 100_000},
	TierGrowth:     {EventsQuota: 10_000_000, QueriesQuota: 1_000_000},
	TierEnterprise: {EventsQuota: -1, QueriesQuota: -1},
	TierTeam:       {EventsQuota: 10_000_000, QueriesQuota: 1_000_000},
}

// QuotasForTier returns the quota limits for the given tier.
// Returns free-tier quotas for unknown tiers.
func QuotasForTier(tier string) TierQuotas {
	t := SubscriptionTier(tier)
	if q, ok := TierQuotaMap[t]; ok {
		return q
	}
	return TierQuotaMap[TierFree]
}

// IsUnlimited returns true if the quota value represents unlimited usage.
func (q TierQuotas) IsUnlimited() bool {
	return q.EventsQuota < 0 && q.QueriesQuota < 0
}

// ToMetadataMap converts a TenantBillingMetadata to a flat map suitable for
// merging into Core's tenant metadata.
func (b *TenantBillingMetadata) ToMetadataMap() map[string]any {
	m := make(map[string]any)
	if b.Subscription != nil {
		m["subscription"] = b.Subscription
	}
	if b.Quotas != nil {
		m["quotas"] = b.Quotas
	}
	if b.Overage != nil {
		m["overage"] = b.Overage
	}
	return m
}
