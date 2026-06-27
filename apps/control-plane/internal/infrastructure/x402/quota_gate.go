package x402

import (
	"context"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// QuotaChecker checks whether a tenant has remaining quota.
// Injected so the x402 package doesn't depend on tenant/billing internals.
type QuotaChecker interface {
	// HasQuota returns true if the tenant has remaining quota for the given route.
	HasQuota(tenantID, routeKey string) bool
}

// TierAllower is an optional interface that QuotaChecker implementations may
// implement to gate x402 priced routes on subscription tier. Per the April 2026
// pricing decision, x402 agent endpoints are a Pro-tier-and-above feature;
// free-tier tenants are rejected before any quota/payment logic runs.
// When a checker does not implement this interface the gate falls open.
type TierAllower interface {
	// AllowsX402 returns true if the tenant's subscription tier is allowed to
	// consume x402 priced routes. Free tier → false.
	AllowsX402(tenantID string) bool
}

// AllowanceChecker is an optional interface a QuotaChecker may implement to
// expose the per-tier included x402 allowance (011 / PRICING_EXPOSURE_PLAN.md
// §2). When a tenant is still inside its included allowance, x402 priced routes
// are served free (no on-chain payment); once the allowance is exhausted the
// gate falls through to the x402 payment flow (pay-as-you-go overage at the
// per-call rate configured in the pricing config).
//
// Separation of concerns: HasQuota tracks the events/queries subscription
// quota; HasX402Allowance tracks the distinct x402 credit bucket. A tenant can
// have events-quota remaining yet still owe per-call x402 once their credit
// allowance is spent — these are independent meters.
type AllowanceChecker interface {
	// HasX402Allowance returns true if the tenant has x402 calls remaining in
	// its included allowance for the current period. A tier with no allowance
	// (allowance == 0) returns false so the gate proceeds to payment. A tier
	// with unlimited allowance (-1) returns true.
	HasX402Allowance(tenantID string) bool
	// RecordAllowanceConsumed notes that one allowance call was just served
	// free, so HasX402Allowance reflects it immediately rather than waiting for
	// the next Core reconciliation tick. Without this, a tenant could overshoot
	// its included allowance by a full reconciliation interval of traffic.
	RecordAllowanceConsumed(tenantID string)
}

// tierAllowedForX402 is the canonical gate list. Per 011 the paid hosted tiers
// (indie, studio, scale) plus enterprise may consume x402 priced routes. The
// retired tiers (pro, growth, team, starter, developer) remain in the list so
// in-flight subscriptions whose stored tenant metadata still carries the old
// tier string are not abruptly denied mid-cutover. Anything else (including
// empty string and "free"/Self-Host) is denied.
var tierAllowedForX402 = map[string]bool{
	// Canonical 011 paid tiers.
	"indie":      true,
	"studio":     true,
	"scale":      true,
	"enterprise": true,
	// Retired aliases — keep until cutover backfill remaps stored metadata.
	"pro":       true,
	"growth":    true,
	"team":      true,
	"starter":   true,
	"developer": true,
}

// QuotaGatedMiddleware only activates x402 payments when the tenant's quota is exceeded.
// - Quota remaining → passthrough (no payment needed)
// - Quota exceeded + X402_ENABLED → x402 payment gate
// - Quota exceeded + X402 disabled → standard 429
func QuotaGatedMiddleware(
	facilitator PaymentFacilitator,
	pricing *PricingConfig,
	logger *EventLogger,
	quotaChecker QuotaChecker,
) gin.HandlerFunc {
	x402Mw := Middleware(facilitator, pricing, logger)

	return func(c *gin.Context) {
		// If x402 is disabled, don't gate at all — let upstream quota enforcement handle it
		if !pricing.Enabled {
			c.Next()
			return
		}

		routeKey := RouteKey(c.Request.Method, c.Request.URL.Path)

		// Only check quota if the route has pricing configured
		if pricing.RequirementsForRoute(routeKey) == nil {
			c.Next()
			return
		}

		// Extract tenant ID
		tenantID, _ := c.Get("tenant_id")
		tenantIDStr, ok := tenantID.(string)
		if !ok {
			tenantIDStr = ""
		}

		// Tier gate: free-tier / Self-Host tenants cannot consume x402 priced
		// routes at all, regardless of quota or payment capability. Paid hosted
		// tiers (indie/studio/scale) and enterprise only.
		if tierAllower, ok := quotaChecker.(TierAllower); ok && tenantIDStr != "" {
			if !tierAllower.AllowsX402(tenantIDStr) {
				c.JSON(http.StatusForbidden, gin.H{
					"error":   "tier_not_allowed",
					"message": "x402 agent endpoints require a paid subscription (Indie or higher). Upgrade at https://all-source.xyz/billing",
				})
				c.Abort()
				return
			}
		}

		// x402 allowance: paid tiers ship an included allowance of x402 calls
		// (50K / 500K / 5M for indie/studio/scale). While the tenant is inside
		// that allowance, priced routes are served free — no on-chain payment.
		// Once exhausted the gate falls through to the x402 payment flow below
		// (pay-as-you-go overage). Tiers without an allowance return false here
		// and proceed straight to payment.
		if allowanceChecker, ok := quotaChecker.(AllowanceChecker); ok && tenantIDStr != "" {
			if allowanceChecker.HasX402Allowance(tenantIDStr) {
				// Record the free allowance call so the counter depletes and
				// overage eventually kicks in. Two records: the durable Core
				// event (reconciled by SyncX402UsageUseCase, source of truth) and
				// the in-process delta (so the very next request on this instance
				// sees the consumption without waiting for the reconciler tick).
				if logger != nil {
					logger.LogAllowanceConsumed(c.Request.Context(), tenantIDStr, routeKey)
				}
				allowanceChecker.RecordAllowanceConsumed(tenantIDStr)
				c.Next()
				return
			}
		}

		// If tenant has events/queries quota remaining, skip x402 — free access
		if tenantIDStr != "" && quotaChecker.HasQuota(tenantIDStr, routeKey) {
			c.Next()
			return
		}

		// Allowance and quota both exhausted — delegate to x402 middleware
		// (pay-as-you-go overage at the per-call rate in the pricing config).
		x402Mw(c)
	}
}

// StaticQuotaChecker always returns the same answer. Used for testing.
// TierDenied lets tests simulate a free-tier tenant hitting an x402 route;
// the zero value (false) means tier gate allows access, matching the
// pre-tier-gate behavior of the quota gate tests.
type StaticQuotaChecker struct {
	HasRemaining bool
	TierDenied   bool
	// AllowanceRemaining lets tests exercise the x402 allowance path. The zero
	// value (false) means "no allowance remaining" so existing tests that don't
	// set it fall through to the quota/payment path unchanged.
	AllowanceRemaining bool
}

// HasQuota implements QuotaChecker.
func (s *StaticQuotaChecker) HasQuota(string, string) bool {
	return s.HasRemaining
}

// HasX402Allowance implements AllowanceChecker for tests.
func (s *StaticQuotaChecker) HasX402Allowance(string) bool {
	return s.AllowanceRemaining
}

// RecordAllowanceConsumed implements AllowanceChecker for tests (no-op).
func (s *StaticQuotaChecker) RecordAllowanceConsumed(string) {}

// CoreQuotaChecker checks tenant quota by reading metadata from Core.
// It maps route keys to quota dimensions: POST routes → events, all others → queries.
type CoreQuotaChecker struct {
	client clients.CoreClient

	// In-process x402 allowance tightening. The scheduler reconciles the durable
	// x402_used counter in Core every minute; between ticks this instance tracks
	// the allowance calls it served free so HasX402Allowance reflects them
	// immediately. Core remains the source of truth (event-sourced).
	//
	// Bound (t-d7aabc): on a SINGLE control-plane instance this is EXACT at the
	// allowance boundary — every free serve increments the local counter before
	// the next check. With N instances the counter is NOT shared, so residual
	// overshoot at the boundary is bounded by (N-1) × calls-served-since-the-last
	// 1-minute reconciler tick across the other instances — far tighter than a
	// full reconciliation window, but non-zero. The Control Plane runs ONE Fly
	// machine today, so the bound is currently zero.
	//
	// Remediation if CP scales out: move this counter to a SHARED store. Prefer a
	// Core-side atomic increment (reconciliation already flows through Core via
	// x402.allowance.consumed events — a Core endpoint that atomically bumps and
	// returns x402_used makes the check exact across instances) over Redis, to
	// avoid adding a second stateful dependency. The operator opt-in env
	// CONTROL_PLANE_MULTI_INSTANCE=true makes this bound LOUD at boot (see
	// main.go) so scaling out can't silently widen it.
	x402mu        sync.Mutex
	x402LocalUsed map[string]int64 // tenantID → calls served free since baseline
	x402Baseline  map[string]int64 // tenantID → Core x402_used at last reset
}

// NewCoreQuotaChecker creates a quota checker backed by Core tenant metadata.
func NewCoreQuotaChecker(client clients.CoreClient) *CoreQuotaChecker {
	return &CoreQuotaChecker{
		client:        client,
		x402LocalUsed: make(map[string]int64),
		x402Baseline:  make(map[string]int64),
	}
}

// HasQuota returns true if the tenant has remaining quota for the given route.
// Returns true on any error (tenant not found, network failure, missing metadata) as a
// safe default — failing open means we never accidentally charge tenants due to transient
// infrastructure issues.
func (q *CoreQuotaChecker) HasQuota(tenantID, routeKey string) bool {
	if q.client == nil {
		return true
	}
	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	tenant, err := q.client.GetTenant(ctx, tenantID)
	if err != nil || tenant == nil {
		return true
	}

	quotaMeta, ok := tenant.Metadata["quota"].(map[string]any)
	if !ok {
		return true
	}

	if strings.HasPrefix(routeKey, "POST ") {
		return quotaRemaining(quotaMeta, "events_used", "events_quota")
	}
	return quotaRemaining(quotaMeta, "queries_used", "queries_quota")
}

// AllowsX402 reads the tenant's subscription tier from Core metadata and
// returns true for pro/growth/enterprise (+ legacy "team"). Fails *closed* on
// any error: unknown tier, missing metadata, or Core unreachable all deny
// access. This is the opposite posture from HasQuota (which fails open to
// avoid surprise charges) — here, failing closed prevents accidentally
// serving paid agent endpoints to unauthenticated/unknown callers.
func (q *CoreQuotaChecker) AllowsX402(tenantID string) bool {
	if q.client == nil || tenantID == "" {
		return false
	}
	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	tenant, err := q.client.GetTenant(ctx, tenantID)
	if err != nil || tenant == nil {
		return false
	}

	subMeta, ok := tenant.Metadata["subscription"].(map[string]any)
	if !ok {
		return false
	}
	tier, ok := subMeta["tier"].(string)
	if !ok {
		return false
	}
	return tierAllowedForX402[tier]
}

// HasX402Allowance reports whether the tenant still has x402 calls remaining in
// its included per-tier allowance. It reads x402_allowance / x402_used from the
// tenant's "quotas" metadata (written by UpdateSubscriptionMetadataUseCase from
// the tier entitlement set). Semantics:
//   - allowance < 0  → unlimited (enterprise) → always true
//   - allowance == 0 → no included allowance → false (proceed to payment)
//   - used < allowance → true (serve free)
//   - used >= allowance → false (allowance spent → overage payment)
//
// Fails *closed* (false) on any error so an unreachable Core never silently
// grants free x402 beyond the allowance — the worst case is a tenant inside
// allowance being asked to pay, which is recoverable, vs. unbounded free usage.
func (q *CoreQuotaChecker) HasX402Allowance(tenantID string) bool {
	if q.client == nil || tenantID == "" {
		return false
	}
	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	tenant, err := q.client.GetTenant(ctx, tenantID)
	if err != nil || tenant == nil {
		return false
	}

	// Billing writes the entitlement bucket under "quotas" (plural).
	quotaMeta, ok := tenant.Metadata["quotas"].(map[string]any)
	if !ok {
		return false
	}

	allowance := metadataInt64(quotaMeta, "x402_allowance")
	if allowance < 0 {
		return true // unlimited (enterprise)
	}
	if allowance == 0 {
		return false // no included allowance for this tier
	}
	coreUsed := metadataInt64(quotaMeta, "x402_used")

	// Add this instance's not-yet-reconciled consumption. When Core's counter
	// changes (reconciler advanced it, or a new period reset it), the local
	// delta is already reflected (or stale) → drop it and rebaseline.
	q.x402mu.Lock()
	if q.x402Baseline[tenantID] != coreUsed {
		q.x402Baseline[tenantID] = coreUsed
		q.x402LocalUsed[tenantID] = 0
	}
	effectiveUsed := coreUsed + q.x402LocalUsed[tenantID]
	q.x402mu.Unlock()

	return effectiveUsed < allowance
}

// RecordAllowanceConsumed increments this instance's local x402 counter for the
// tenant. Called by the gate immediately after an allowance call is served free,
// so the next request sees the consumption before the Core reconciler catches up.
func (q *CoreQuotaChecker) RecordAllowanceConsumed(tenantID string) {
	if tenantID == "" {
		return
	}
	q.x402mu.Lock()
	q.x402LocalUsed[tenantID]++
	q.x402mu.Unlock()
}

// HasExtractionQuota reports whether the tenant still has hosted Hound
// doc-extraction LLM tokens remaining in its included per-tier allowance. It
// reads extraction_tokens_quota / extraction_tokens_used from the tenant's
// "quotas" metadata (the allowance is written from the tier entitlement by
// UpdateSubscriptionMetadataUseCase; the meter is reconciled from
// prime.extraction.usage events by billing.SyncExtractionUsageUseCase).
// Semantics mirror the events_used gate:
//   - quota < 0   → unlimited (enterprise) → true
//   - quota == 0  → no included allowance (free tier = BYO only) → false (blocked)
//   - used < quota → true
//   - used >= quota → false (blocked)
//
// Fails *open* (true) on any error, matching HasQuota: a transient Core outage
// must not block a legitimate tenant's extraction. A hosted-extraction request
// path calls this and returns 402/blocked when it is false.
func (q *CoreQuotaChecker) HasExtractionQuota(tenantID string) bool {
	if q.client == nil || tenantID == "" {
		return true
	}
	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()

	tenant, err := q.client.GetTenant(ctx, tenantID)
	if err != nil || tenant == nil {
		return true
	}
	// Billing writes the entitlement + meter under "quotas" (plural).
	quotaMeta, ok := tenant.Metadata["quotas"].(map[string]any)
	if !ok {
		return true
	}
	return quotaRemaining(quotaMeta, "extraction_tokens_used", "extraction_tokens_quota")
}

// AllowsX402 implements TierAllower for the static test checker.
// Defaults to allow (TierDenied == false) so pre-existing tests that only
// exercise quota/payment paths continue to work without modification.
func (s *StaticQuotaChecker) AllowsX402(string) bool {
	return !s.TierDenied
}

func quotaRemaining(m map[string]any, usedKey, quotaKey string) bool {
	quota := metadataInt64(m, quotaKey)
	if quota < 0 {
		return true // negative quota means no limit
	}
	used := metadataInt64(m, usedKey)
	return used < quota
}

// metadataInt64 extracts an int64 from a metadata map value.
// JSON unmarshalling produces float64 for all numbers; int/int64 variants are also handled.
func metadataInt64(m map[string]any, key string) int64 {
	switch v := m[key].(type) {
	case float64:
		return int64(v)
	case int64:
		return v
	case int:
		return int64(v)
	default:
		return 0
	}
}

// QuotaExceededResponse returns a standard 429 when x402 is disabled.
func QuotaExceededResponse(c *gin.Context) {
	c.JSON(http.StatusTooManyRequests, gin.H{
		"error":   "quota_exceeded",
		"message": "Usage quota exceeded. Upgrade your plan or enable x402 pay-per-use.",
	})
	c.Abort()
}
