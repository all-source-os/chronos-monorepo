package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// coreGateway is the narrow slice of the Core client the email webhook needs.
// clients.CoreClient satisfies it; tests provide a fake.
type coreGateway interface {
	GetConfig(ctx context.Context, key string) (*clients.ConfigEntryResponse, error)
	IngestEvent(ctx context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error)
}

// EmailWebhookHandler turns provider email webhooks into durable email.* Core
// events. It terminates the public webhook (Core never authenticates): verify
// signature -> resolve tenant -> fetch the full message -> ingest email.received.
type EmailWebhookHandler struct {
	provider emailprovider.Provider
	core     coreGateway
	// sealer decrypts the per-grant record stored in Core config. nil keeps the
	// legacy plaintext path working during migration (P3a).
	sealer *secrets.Sealer
}

// NewEmailWebhookHandler builds the handler. provider may be nil when the email
// connector is not configured (no NYLAS_API_KEY); the handler then returns 503.
// sealer may be nil when CONNECTOR_SECRET_KEY is unset (legacy plaintext only).
func NewEmailWebhookHandler(provider emailprovider.Provider, core coreGateway, sealer *secrets.Sealer) *EmailWebhookHandler {
	return &EmailWebhookHandler{provider: provider, core: core, sealer: sealer}
}

// grantConfigKey maps a provider grant id to the Core config key holding its
// tenant id (written at grant onboarding; per-tenant isolation in P3).
func grantConfigKey(grantID string) string {
	return "connector:email:grant:" + grantID
}

// nylasNotification is the subset of a Nylas v3 webhook delivery we consume.
type nylasNotification struct {
	Type string `json:"type"`
	Data struct {
		Object struct {
			ID      string `json:"id"`
			GrantID string `json:"grant_id"`
		} `json:"object"`
	} `json:"data"`
}

// EmailChallenge answers the provider's GET webhook-verification handshake by
// echoing the ?challenge= parameter. Route: GET /api/v1/webhooks/email
func (h *EmailWebhookHandler) EmailChallenge(c *gin.Context) {
	challenge := c.Query("challenge")
	if challenge == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing challenge"})
		return
	}
	c.String(http.StatusOK, challenge)
}

// Email handles POST /api/v1/webhooks/email.
func (h *EmailWebhookHandler) Email(c *gin.Context) {
	if h.provider == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "email connector not configured"})
		return
	}

	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "failed to read request body"})
		return
	}

	// Provider signs deliveries with HMAC-SHA256 (Nylas: X-Nylas-Signature).
	if !h.provider.VerifySignature(body, c.GetHeader("X-Nylas-Signature")) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid signature"})
		return
	}

	var note nylasNotification
	if err := json.Unmarshal(body, &note); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid JSON payload"})
		return
	}

	// P0 handles inbound message creation; other triggers are acked + ignored.
	if note.Type != "message.created" {
		c.JSON(http.StatusOK, gin.H{"status": "ignored", "type": note.Type})
		return
	}
	grantID, messageID := note.Data.Object.GrantID, note.Data.Object.ID
	if grantID == "" || messageID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing grant_id or message id"})
		return
	}

	ctx := c.Request.Context()

	// Resolve tenant from the grant. Unknown grant -> ack + ignore (no retry storm).
	tenantID := h.resolveTenant(ctx, grantID)
	if tenantID == "" {
		c.JSON(http.StatusOK, gin.H{"status": "ignored", "reason": "unknown grant"})
		return
	}

	// P2 (t-b773e6): allowance / x402 metering gates here, fail-closed. The P0
	// wedge is single-tenant, so the gate is currently a pass-through.

	msg, err := h.provider.FetchMessage(ctx, grantID, messageID)
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "fetch message failed"})
		return
	}

	payload, err := toMap(msg.ToReceivedPayload())
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "encode payload failed"})
		return
	}

	var firstIngest uint64
	_, err = h.core.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "email.received",
		EntityID:  messageID, // provider message id; lifecycle events share it
		TenantID:  tenantID,  // resolved server-side, never client-trusted
		Payload:   payload,
		Metadata: map[string]any{
			"provider":        h.provider.Name(),
			"grant_id":        grantID,
			"idempotency_key": messageID,
		},
		ExpectedVersion: &firstIngest, // 0: first-ingest dedupe (§4.6)
	})
	if err != nil {
		// A version conflict means this message was already ingested: idempotent.
		if errors.Is(err, clients.ErrVersionConflict) {
			c.JSON(http.StatusOK, gin.H{"status": "duplicate"})
			return
		}
		c.JSON(http.StatusInternalServerError, gin.H{"error": "ingest failed"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "ingested"})
}

// resolveTenant reads the tenant id from the per-grant record in Core config.
// The record is a sealed JSON blob ({tenant_id, grant_id, ...}); when a sealer
// is configured it is decrypted first. A non-sealed (legacy plaintext) value is
// accepted during migration. Returns "" when the grant is unknown or the sealed
// value cannot be opened (fail closed).
func (h *EmailWebhookHandler) resolveTenant(ctx context.Context, grantID string) string {
	cfg, err := h.core.GetConfig(ctx, grantConfigKey(grantID))
	if err != nil || cfg == nil {
		return ""
	}
	switch v := cfg.Value.(type) {
	case string:
		if h.sealer != nil {
			plain, err := h.sealer.Open(v)
			switch {
			case err == nil:
				var rec struct {
					TenantID string `json:"tenant_id"`
				}
				if json.Unmarshal(plain, &rec) == nil {
					return rec.TenantID
				}
				return ""
			case errors.Is(err, secrets.ErrNotSealed):
				// legacy plaintext tenant id — fall through
			default:
				return "" // sealed but undecryptable: fail closed
			}
		}
		return v
	case map[string]any:
		if t, ok := v["tenant_id"].(string); ok {
			return t
		}
	}
	return ""
}

// toMap round-trips a value through JSON into a generic map for the Core payload.
func toMap(v any) (map[string]any, error) {
	b, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		return nil, err
	}
	return m, nil
}
