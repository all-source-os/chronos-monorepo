package x402

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

// Middleware returns a gin middleware that enforces x402 payments on configured routes.
// If the route has no pricing configured, the request passes through.
// If pricing exists and no X-Payment header is present, returns HTTP 402.
// If X-Payment header is present, verifies and settles the payment.
func Middleware(facilitator PaymentFacilitator, pricing *PricingConfig, logger *EventLogger) gin.HandlerFunc {
	return func(c *gin.Context) {
		if !pricing.Enabled {
			c.Next()
			return
		}

		routeKey := RouteKey(c.Request.Method, c.Request.URL.Path)
		requirements := pricing.RequirementsForRoute(routeKey)
		if requirements == nil {
			// No pricing for this route — pass through
			c.Next()
			return
		}

		// Extract tenant ID from context (set by auth middleware upstream)
		tenantID, _ := c.Get("tenant_id")
		tenantIDStr, ok := tenantID.(string)
		if !ok {
			tenantIDStr = ""
		}

		// Check for payment header
		paymentHeader := c.GetHeader(HeaderPayment)
		if paymentHeader == "" {
			// No payment — return 402 with requirements
			if logger != nil && tenantIDStr != "" {
				firstAccept := requirements.Accepts[0]
				logger.LogRequested(c.Request.Context(), tenantIDStr, routeKey, firstAccept.Network, firstAccept.Amount)
			}

			encoded, err := EncodeHeader(requirements)
			if err != nil {
				c.JSON(http.StatusInternalServerError, gin.H{"error": "encode_requirements_failed"})
				c.Abort()
				return
			}

			c.Header(HeaderPayment, encoded)
			c.JSON(http.StatusPaymentRequired, requirements)
			c.Abort()
			return
		}

		// Decode payment from header
		var payment PaymentPayload
		if err := DecodeHeader(paymentHeader, &payment); err != nil {
			c.JSON(http.StatusBadRequest, gin.H{
				"error":   "invalid_payment",
				"message": "Failed to decode X-Payment header: " + err.Error(),
			})
			c.Abort()
			return
		}

		// Settle (verify + submit on-chain)
		settleResp, err := facilitator.Settle(c.Request.Context(), &payment, requirements)
		if err != nil {
			if logger != nil && tenantIDStr != "" {
				logger.LogFailed(c.Request.Context(), tenantIDStr, err.Error(), payment.Accepted.Network, routeKey)
			}
			c.JSON(http.StatusInternalServerError, gin.H{
				"error":   "settlement_error",
				"message": err.Error(),
			})
			c.Abort()
			return
		}

		if !settleResp.Success {
			if logger != nil && tenantIDStr != "" {
				logger.LogFailed(c.Request.Context(), tenantIDStr, settleResp.ErrorReason, payment.Accepted.Network, routeKey)
			}
			c.JSON(http.StatusPaymentRequired, gin.H{
				"error":   settleResp.ErrorReason,
				"message": settleResp.ErrorMessage,
			})
			c.Abort()
			return
		}

		// Payment settled — log and attach receipt header
		if logger != nil && tenantIDStr != "" {
			logger.LogSettled(c.Request.Context(), tenantIDStr, settleResp.Transaction, settleResp.Payer, settleResp.Network, payment.Accepted.Amount, routeKey)
		}

		receiptEncoded, err := EncodeHeader(SettlementResponse{
			Success:     true,
			Transaction: settleResp.Transaction,
			Network:     settleResp.Network,
			Payer:       settleResp.Payer,
		})
		if err == nil {
			c.Header(HeaderPaymentResponse, receiptEncoded)
		}

		// Payment accepted — continue to handler
		c.Next()
	}
}
