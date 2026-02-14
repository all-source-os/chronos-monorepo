package http //nolint:revive // package name intentionally matches directory

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
)

// WebhookHandler handles incoming webhook HTTP requests.
type WebhookHandler struct {
	processWebhookUC *usecases.ProcessLemonSqueezyWebhookUseCase
}

// NewWebhookHandler creates a new WebhookHandler.
func NewWebhookHandler(processWebhookUC *usecases.ProcessLemonSqueezyWebhookUseCase) *WebhookHandler {
	return &WebhookHandler{
		processWebhookUC: processWebhookUC,
	}
}

// LemonSqueezy handles POST /api/v1/webhooks/lemonsqueezy
func (h *WebhookHandler) LemonSqueezy(c *gin.Context) {
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
	if err := h.processWebhookUC.Execute(c.Request.Context(), event); err != nil {
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
