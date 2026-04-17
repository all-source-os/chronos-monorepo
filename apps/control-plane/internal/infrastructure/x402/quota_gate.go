package x402

import (
	"context"
	"net/http"
	"strings"
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

// tierAllowedForX402 is the canonical gate list — pro, growth, enterprise, and
// the legacy "team" alias. Anything else (including empty string and "free")
// is denied.
var tierAllowedForX402 = map[string]bool{
	"pro":        true,
	"growth":     true,
	"enterprise": true,
	"team":       true, // legacy alias for growth
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

		// Tier gate: free-tier tenants cannot consume x402 priced routes at
		// all, regardless of quota or payment capability. Pro-and-above only.
		if tierAllower, ok := quotaChecker.(TierAllower); ok && tenantIDStr != "" {
			if !tierAllower.AllowsX402(tenantIDStr) {
				c.JSON(http.StatusForbidden, gin.H{
					"error":   "tier_not_allowed",
					"message": "x402 agent endpoints require a Pro subscription or higher. Upgrade at https://all-source.xyz/billing",
				})
				c.Abort()
				return
			}
		}

		// If tenant has quota remaining, skip x402 — free access
		if tenantIDStr != "" && quotaChecker.HasQuota(tenantIDStr, routeKey) {
			c.Next()
			return
		}

		// Quota exceeded — delegate to x402 middleware
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
}

// HasQuota implements QuotaChecker.
func (s *StaticQuotaChecker) HasQuota(string, string) bool {
	return s.HasRemaining
}

// CoreQuotaChecker checks tenant quota by reading metadata from Core.
// It maps route keys to quota dimensions: POST routes → events, all others → queries.
type CoreQuotaChecker struct {
	client clients.CoreClient
}

// NewCoreQuotaChecker creates a quota checker backed by Core tenant metadata.
func NewCoreQuotaChecker(client clients.CoreClient) *CoreQuotaChecker {
	return &CoreQuotaChecker{client: client}
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
