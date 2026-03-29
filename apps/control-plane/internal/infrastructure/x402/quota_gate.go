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
type StaticQuotaChecker struct {
	HasRemaining bool
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
