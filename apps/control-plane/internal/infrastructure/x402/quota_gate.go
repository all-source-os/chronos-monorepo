package x402

import (
	"net/http"

	"github.com/gin-gonic/gin"
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

// QuotaExceededResponse returns a standard 429 when x402 is disabled.
func QuotaExceededResponse(c *gin.Context) {
	c.JSON(http.StatusTooManyRequests, gin.H{
		"error":   "quota_exceeded",
		"message": "Usage quota exceeded. Upgrade your plan or enable x402 pay-per-use.",
	})
	c.Abort()
}
