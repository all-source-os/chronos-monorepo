package dto

import "time"

// AdminTenantDetailResponse represents the full admin view of a tenant,
// including plan, quotas, members, and subscription metadata.
type AdminTenantDetailResponse struct {
	ID          string                 `json:"id"`
	Name        string                 `json:"name"`
	Description string                 `json:"description"`
	Status      string                 `json:"status"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
	Metadata    map[string]interface{} `json:"metadata"`

	// Plan and quota information
	Plan   PlanInfo             `json:"plan"`
	Quotas TenantQuotasResponse `json:"quotas"`

	// Subscription metadata (from payment provider)
	Subscription *SubscriptionInfo `json:"subscription,omitempty"`

	// Usage summary — real per-tenant counts from Core metering.
	// EventCount is the tenant's event total from GET /api/v1/tenants/{id}/stats;
	// MemberCount is the size of the real team-members list. Both are guarded 0
	// for a zero-usage tenant, never omitted.
	EventCount  int64 `json:"event_count"`
	MemberCount int   `json:"member_count"`
}

// PlanInfo holds plan-level details for a tenant.
type PlanInfo struct {
	Name string `json:"name"`
	Tier string `json:"tier"`
}

// SubscriptionInfo holds payment/subscription metadata.
type SubscriptionInfo struct {
	Provider   string     `json:"provider"`
	ExternalID string     `json:"external_id"`
	Status     string     `json:"status"`
	PlanName   string     `json:"plan_name,omitempty"`
	StartedAt  *time.Time `json:"started_at,omitempty"`
	RenewsAt   *time.Time `json:"renews_at,omitempty"`
}

// TenantUsageResponse represents the usage breakdown for a tenant.
type TenantUsageResponse struct {
	TenantID     string `json:"tenant_id"`
	EventCount   int64  `json:"event_count"`
	QueryCount   int64  `json:"query_count"`
	StorageBytes int64  `json:"storage_bytes"`
	StreamCount  int64  `json:"stream_count"`
	// Quota limits, sourced from the tenant's metadata.quotas. Without these the
	// admin Usage-trends bars have no denominator and render "0 / 0 (0.0%)".
	EventLimit     int64        `json:"event_limit"`
	QueryLimit     int64        `json:"query_limit"`
	StorageLimitMB int64        `json:"storage_limit_mb"`
	DailyUsage     []DailyUsage `json:"daily_usage"`
}

// DailyUsage represents per-day usage metrics.
type DailyUsage struct {
	Date           string `json:"date"`
	EventsIngested int64  `json:"events_ingested"`
	QueriesRun     int64  `json:"queries_run"`
	StorageBytes   int64  `json:"storage_bytes"`
}
