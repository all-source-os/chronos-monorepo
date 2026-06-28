package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"log"
	"net/http"
	"strings"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// resendInbound is the slice of *resend.Provider the inbound handler needs.
// A real *resend.Provider satisfies it; tests provide a fake.
type resendInbound interface {
	VerifyWebhook(headers http.Header, body []byte) bool
	FetchMessage(ctx context.Context, grantID, emailID string) (*emailprovider.Message, error)
	Name() string
}

// ResendWebhookHandler turns Resend inbound webhooks into durable email.received
// Core events. Resend's webhook is thin (email_id only), Svix-signed; we verify,
// resolve the tenant by the registered To address, fetch the full message, and
// ingest. Tenant resolution + the sealed connection record reuse the same
// grant-config machinery as the Nylas path (grant_id ≡ the receiving address).
type ResendWebhookHandler struct {
	provider resendInbound
	core     coreGateway // GetConfig + IngestEvent (defined in email_webhook_handler.go)
	sealer   *secrets.Sealer
}

// NewResendWebhookHandler builds the handler. provider may be nil when Resend is
// not configured (no RESEND_API_KEY) → the handler returns 503.
func NewResendWebhookHandler(provider resendInbound, core coreGateway, sealer *secrets.Sealer) *ResendWebhookHandler {
	return &ResendWebhookHandler{provider: provider, core: core, sealer: sealer}
}

// resendInboundNotification is the subset of a Resend email.received webhook we
// consume. The body/headers are NOT in the webhook — only the id + recipients.
type resendInboundNotification struct {
	Type string `json:"type"`
	Data struct {
		EmailID string   `json:"email_id"`
		To      []string `json:"to"`
	} `json:"data"`
}

// Inbound handles POST /api/v1/webhooks/resend/inbound.
func (h *ResendWebhookHandler) Inbound(c *gin.Context) {
	if h.provider == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "resend connector not configured"})
		return
	}
	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "failed to read request body"})
		return
	}
	// Svix HMAC over the id+timestamp+body, keyed by the whsec_ secret.
	if !h.provider.VerifyWebhook(c.Request.Header, body) {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid signature"})
		return
	}

	var note resendInboundNotification
	if err := json.Unmarshal(body, &note); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid JSON payload"})
		return
	}
	if note.Type != "email.received" {
		c.JSON(http.StatusOK, gin.H{"status": "ignored", "type": note.Type})
		return
	}
	if note.Data.EmailID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing email_id"})
		return
	}

	ctx := c.Request.Context()
	// Resolve the tenant by the first registered recipient address.
	tenantID, addr := "", ""
	for _, t := range note.Data.To {
		a := strings.ToLower(emailOf(t))
		if a == "" {
			continue
		}
		if tid := h.resolveTenant(ctx, a); tid != "" {
			tenantID, addr = tid, a
			break
		}
	}
	if tenantID == "" {
		c.JSON(http.StatusOK, gin.H{"status": "ignored", "reason": "unknown recipient"})
		return
	}

	msg, err := h.provider.FetchMessage(ctx, addr, note.Data.EmailID)
	if err != nil {
		log.Printf("resend inbound: fetch failed email_id=%s: %v", note.Data.EmailID, err)
		c.JSON(http.StatusBadGateway, gin.H{"error": "fetch message failed"})
		return
	}

	payload, err := toMap(msg.ToReceivedPayload())
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "encode payload failed"})
		return
	}
	// Stamp the mailbox (address) so the stream can scope to a single mailbox.
	payload["grant_id"] = addr

	var firstIngest uint64
	_, err = h.core.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "email.received",
		EntityID:  msg.ID, // RFC Message-ID — stable + what a reply's In-Reply-To references
		TenantID:  tenantID,
		Payload:   payload,
		Metadata: map[string]any{
			"provider":        h.provider.Name(),
			"grant_id":        addr,
			"idempotency_key": msg.ID,
		},
		ExpectedVersion: &firstIngest, // first-ingest dedupe
	})
	if err != nil {
		if errors.Is(err, clients.ErrVersionConflict) {
			c.JSON(http.StatusOK, gin.H{"status": "duplicate"})
			return
		}
		log.Printf("resend inbound: ingest failed: %v", err)
		c.JSON(http.StatusInternalServerError, gin.H{"error": "ingest failed"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "ingested"})
}

// resolveTenant reads the tenant id from the sealed connection record stored in
// Core config under the receiving address. Fail-closed.
func (h *ResendWebhookHandler) resolveTenant(ctx context.Context, addr string) string {
	cfg, err := h.core.GetConfig(ctx, grantConfigKey(addr))
	if err != nil || cfg == nil {
		return ""
	}
	s, ok := cfg.Value.(string)
	if !ok || h.sealer == nil {
		return ""
	}
	plain, err := h.sealer.Open(s)
	if err != nil {
		return ""
	}
	var rec struct {
		TenantID string `json:"tenant_id"`
	}
	if json.Unmarshal(plain, &rec) == nil {
		return rec.TenantID
	}
	return ""
}

// emailOf extracts the bare address from "Name <email@x.com>" or "email@x.com".
func emailOf(s string) string {
	s = strings.TrimSpace(s)
	if i := strings.LastIndex(s, "<"); i >= 0 {
		if j := strings.IndexByte(s[i:], '>'); j >= 0 {
			return strings.TrimSpace(s[i+1 : i+j])
		}
	}
	return s
}
