package http //nolint:revive // package name intentionally matches directory

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// WebhookHandler handles incoming webhook HTTP requests.
type WebhookHandler struct {
	processLSWebhookUC     *usecases.ProcessLemonSqueezyWebhookUseCase
	processStripeWebhookUC *usecases.ProcessStripeWebhookUseCase
}

// NewWebhookHandler creates a new WebhookHandler.
func NewWebhookHandler(
	processLSWebhookUC *usecases.ProcessLemonSqueezyWebhookUseCase,
	processStripeWebhookUC *usecases.ProcessStripeWebhookUseCase,
) *WebhookHandler {
	return &WebhookHandler{
		processLSWebhookUC:     processLSWebhookUC,
		processStripeWebhookUC: processStripeWebhookUC,
	}
}

// LemonSqueezy handles POST /api/v1/webhooks/lemonsqueezy
func (h *WebhookHandler) LemonSqueezy(c *gin.Context) { //nolint:dupl // each webhook handler has different signature verification and event types
	// Read the raw body for HMAC verification
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "failed to read request body"})
		return
	}

	// Verify HMAC signature
	signature := c.GetHeader("X-Signature")
	if signature == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing X-Signature header"})
		return
	}

	secret := os.Getenv("LEMON_SQUEEZY_WEBHOOK_SECRET")
	if secret == "" {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "webhook secret not configured"})
		return
	}

	if !verifyHMACSignature(body, signature, secret) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid signature"})
		return
	}

	// Parse the webhook event
	var event usecases.LemonSqueezyWebhookEvent
	if err := json.Unmarshal(body, &event); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid JSON payload"})
		return
	}

	// Process the event
	if err := h.processLSWebhookUC.Execute(c.Request.Context(), event); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"status": "processed"})
}

// Stripe handles POST /api/v1/webhooks/stripe
func (h *WebhookHandler) Stripe(c *gin.Context) { //nolint:dupl // each webhook handler has different signature verification and event types
	// Read the raw body for signature verification
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "failed to read request body"})
		return
	}

	// Verify Stripe signature (Stripe-Signature header)
	sigHeader := c.GetHeader("Stripe-Signature")
	if sigHeader == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing Stripe-Signature header"})
		return
	}

	secret := os.Getenv("STRIPE_WEBHOOK_SECRET")
	if secret == "" {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "webhook secret not configured"})
		return
	}

	if !verifyStripeSignature(body, sigHeader, secret) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid signature"})
		return
	}

	// Parse the webhook event
	var event usecases.StripeWebhookEvent
	if err := json.Unmarshal(body, &event); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid JSON payload"})
		return
	}

	// Process the event
	if err := h.processStripeWebhookUC.Execute(c.Request.Context(), event); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": err.Error()})
		return
	}

	c.JSON(http.StatusOK, gin.H{"status": "processed"})
}

// verifyHMACSignature verifies the LemonSqueezy HMAC-SHA256 signature.
func verifyHMACSignature(payload []byte, signature, secret string) bool {
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(payload)
	expected := hex.EncodeToString(mac.Sum(nil))
	return hmac.Equal([]byte(expected), []byte(signature))
}

// verifyStripeSignature verifies the Stripe webhook signature.
// Stripe-Signature header format: t=timestamp,v1=signature
func verifyStripeSignature(payload []byte, sigHeader, secret string) bool {
	// Parse the signature header
	var timestamp, sig string
	for _, part := range strings.Split(sigHeader, ",") {
		kv := strings.SplitN(part, "=", 2)
		if len(kv) != 2 {
			continue
		}
		switch kv[0] {
		case "t":
			timestamp = kv[1]
		case "v1":
			sig = kv[1]
		}
	}
	if timestamp == "" || sig == "" {
		return false
	}

	// Compute expected signature: HMAC-SHA256(secret, "timestamp.payload")
	signedPayload := fmt.Sprintf("%s.%s", timestamp, string(payload))
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(signedPayload))
	expected := hex.EncodeToString(mac.Sum(nil))

	return hmac.Equal([]byte(expected), []byte(sig))
}
