package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// inboxCore is the slice of the Core client the admin inbox endpoints need.
// clients.CoreClient satisfies it; tests provide a fake.
type inboxCore interface {
	ListConfigs(ctx context.Context) (*clients.ListConfigsResponse, error)
	DeleteConfig(ctx context.Context, key string) error
	IngestEvent(ctx context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error)
	QueryEvents(ctx context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error)
}

// inboxSender is the slice of the email provider the send endpoint needs.
// *nylas.Provider satisfies it.
type inboxSender interface {
	Name() string
	Send(ctx context.Context, grantID string, req emailprovider.SendRequest) (*emailprovider.SendResult, error)
}

// InboxAdminHandler exposes the AI inbox over admin HTTP so a dashboard can
// manage it: list/disconnect connections (sealed grants), read the email.*
// message stream from Core, and triage/draft/send (the HTTP twins of the
// prime-mcp inbox verbs — epic P1). All routes are admin-gated.
type InboxAdminHandler struct {
	core   inboxCore
	sealer *secrets.Sealer
	sender inboxSender // may be nil when no provider is configured
}

// NewInboxAdminHandler builds the handler. core/sealer nil → endpoints 503.
func NewInboxAdminHandler(core inboxCore, sealer *secrets.Sealer, sender inboxSender) *InboxAdminHandler {
	return &InboxAdminHandler{core: core, sealer: sealer, sender: sender}
}

func (h *InboxAdminHandler) configured() bool { return h.core != nil && h.sealer != nil }

// grantRecord is the decrypted per-grant connection record stored in Core config.
type grantRecord struct {
	TenantID    string `json:"tenant_id"`
	GrantID     string `json:"grant_id"`
	Email       string `json:"email"`
	Provider    string `json:"provider"`
	ConnectedAt string `json:"connected_at"`
}

// openGrant decrypts a sealed Core-config value into a grantRecord.
func (h *InboxAdminHandler) openGrant(value any) (*grantRecord, bool) {
	s, ok := value.(string)
	if !ok {
		return nil, false
	}
	plain, err := h.sealer.Open(s)
	if err != nil {
		return nil, false
	}
	var rec grantRecord
	if json.Unmarshal(plain, &rec) != nil {
		return nil, false
	}
	return &rec, true
}

// connections returns every decrypted connection, optionally filtered by tenant.
func (h *InboxAdminHandler) connections(ctx context.Context, tenantFilter string) ([]grantRecord, error) {
	res, err := h.core.ListConfigs(ctx)
	if err != nil {
		return nil, err
	}
	out := []grantRecord{}
	for _, e := range res.Configs {
		if !strings.HasPrefix(e.Key, "connector:email:grant:") {
			continue
		}
		rec, ok := h.openGrant(e.Value)
		if !ok {
			continue // skip (and don't leak) an unopenable sealed value
		}
		if tenantFilter != "" && rec.TenantID != tenantFilter {
			continue
		}
		out = append(out, *rec)
	}
	return out, nil
}

// ListConnections handles GET /api/v1/admin/inbox/connections[?tenant_id=].
func (h *InboxAdminHandler) ListConnections(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	conns, err := h.connections(c.Request.Context(), c.Query("tenant_id"))
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to list connections"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"connections": conns, "count": len(conns)})
}

// Disconnect handles DELETE /api/v1/admin/inbox/connections/:grant_id.
func (h *InboxAdminHandler) Disconnect(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	grantID := c.Param("grant_id")
	if grantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing grant_id"})
		return
	}
	if err := h.core.DeleteConfig(c.Request.Context(), grantConfigKey(grantID)); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to disconnect"})
		return
	}
	// NOTE: provider-side grant revocation (Nylas) is a best-effort TODO.
	c.JSON(http.StatusOK, gin.H{"status": "disconnected", "grant_id": grantID})
}

// Messages handles GET /api/v1/admin/inbox/messages?tenant_id=&limit=. The inbox
// view's data source is Core email.* events — not a new store.
func (h *InboxAdminHandler) Messages(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing 'tenant_id'"})
		return
	}
	limit := 50
	if v := c.Query("limit"); v != "" {
		if n, err := strconv.Atoi(v); err == nil && n > 0 && n <= 500 {
			limit = n
		}
	}
	res, err := h.core.QueryEvents(c.Request.Context(), clients.QueryEventsRequest{
		EventType: "email.", // prefix match
		TenantID:  tenantID,
		Order:     "desc",
		Limit:     limit,
	})
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to query messages"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"messages": res.Events, "count": res.Count})
}

func by(v string) string {
	if v == "human" {
		return "human"
	}
	return "claude"
}

// Triage handles POST /api/v1/admin/inbox/triage — writes an email.triaged event.
func (h *InboxAdminHandler) Triage(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	var req struct {
		TenantID  string `json:"tenant_id"`
		MessageID string `json:"message_id"`
		ThreadID  string `json:"thread_id"`
		Label     string `json:"label"`
		Reason    string `json:"reason"`
		By        string `json:"by"`
	}
	if c.ShouldBindJSON(&req) != nil || req.TenantID == "" || req.MessageID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id, message_id and label are required"})
		return
	}
	switch req.Label {
	case "needs-reply", "fyi", "spam", "archive":
	default:
		c.JSON(http.StatusBadRequest, gin.H{"error": "label must be one of needs-reply|fyi|spam|archive"})
		return
	}
	payload := map[string]any{"label": req.Label, "by": by(req.By), "triaged_at": time.Now().UTC().Format(time.RFC3339)}
	if req.ThreadID != "" {
		payload["thread_id"] = req.ThreadID
	}
	if req.Reason != "" {
		payload["reason"] = req.Reason
	}
	if _, err := h.core.IngestEvent(c.Request.Context(), clients.IngestEventRequest{
		EventType: "email.triaged",
		EntityID:  req.MessageID,
		TenantID:  req.TenantID,
		Payload:   payload,
	}); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to write triage"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "triaged", "message_id": req.MessageID, "label": req.Label})
}

func newID(prefix string) string {
	b := make([]byte, 16)
	_, _ = rand.Read(b)
	return prefix + hex.EncodeToString(b)
}

// Draft handles POST /api/v1/admin/inbox/draft — writes an email.drafted event.
func (h *InboxAdminHandler) Draft(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	var req struct {
		TenantID  string                  `json:"tenant_id"`
		ThreadID  string                  `json:"thread_id"`
		Body      string                  `json:"body"`
		Intent    string                  `json:"intent"`
		InReplyTo string                  `json:"in_reply_to"`
		Subject   string                  `json:"subject"`
		To        []emailprovider.Address `json:"to"`
		By        string                  `json:"by"`
	}
	if c.ShouldBindJSON(&req) != nil || req.TenantID == "" || req.ThreadID == "" || req.Body == "" || req.Intent == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id, thread_id, body and intent are required"})
		return
	}
	draftID := newID("draft_")
	payload := map[string]any{
		"thread_id":  req.ThreadID,
		"body":       req.Body,
		"intent":     req.Intent,
		"by":         by(req.By),
		"drafted_at": time.Now().UTC().Format(time.RFC3339),
	}
	if req.InReplyTo != "" {
		payload["in_reply_to"] = req.InReplyTo
	}
	if req.Subject != "" {
		payload["subject"] = req.Subject
	}
	if len(req.To) > 0 {
		payload["to"] = req.To
	}
	if _, err := h.core.IngestEvent(c.Request.Context(), clients.IngestEventRequest{
		EventType: "email.drafted",
		EntityID:  draftID,
		TenantID:  req.TenantID,
		Payload:   payload,
		Metadata:  map[string]any{"draft_id": draftID},
	}); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to write draft"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "drafted", "draft_id": draftID})
}

// Send handles POST /api/v1/admin/inbox/send — sends through the tenant's mailbox
// (Nylas) and writes an email.sent event. Confirm-gated; never sends on a bare call.
func (h *InboxAdminHandler) Send(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	if h.sender == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "email provider not configured"})
		return
	}
	var req struct {
		TenantID  string                  `json:"tenant_id"`
		ThreadID  string                  `json:"thread_id"`
		InReplyTo string                  `json:"in_reply_to"`
		Subject   string                  `json:"subject"`
		Body      string                  `json:"body"`
		To        []emailprovider.Address `json:"to"`
		DraftID   string                  `json:"draft_id"`
		Confirm   bool                    `json:"confirm"`
	}
	if c.ShouldBindJSON(&req) != nil || req.TenantID == "" || req.Body == "" || len(req.To) == 0 {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id, to and body are required"})
		return
	}
	if !req.Confirm {
		c.JSON(http.StatusBadRequest, gin.H{"error": "send requires confirm:true"})
		return
	}

	// Resolve the tenant's connected grant (the mailbox we send through).
	conns, err := h.connections(c.Request.Context(), req.TenantID)
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to resolve connection"})
		return
	}
	if len(conns) == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "no connected inbox for tenant"})
		return
	}
	grantID := conns[0].GrantID

	result, err := h.sender.Send(c.Request.Context(), grantID, emailprovider.SendRequest{
		ThreadID:  req.ThreadID,
		InReplyTo: req.InReplyTo,
		Subject:   req.Subject,
		To:        req.To,
		Body:      req.Body,
	})
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "send failed"})
		return
	}

	meta := map[string]any{"provider": h.sender.Name(), "grant_id": grantID}
	if req.DraftID != "" {
		meta["draft_id"] = req.DraftID
	}
	if _, err := h.core.IngestEvent(c.Request.Context(), clients.IngestEventRequest{
		EventType: "email.sent",
		EntityID:  result.MessageID,
		TenantID:  req.TenantID,
		Payload: map[string]any{
			"thread_id": result.ThreadID,
			"subject":   req.Subject,
			"to":        req.To,
			"sent_at":   result.SentAt.UTC().Format(time.RFC3339),
			"direction": "outbound",
		},
		Metadata: meta,
	}); err != nil {
		// The mail was sent; failing to record the event is logged, not fatal to the caller.
		c.JSON(http.StatusOK, gin.H{"status": "sent", "message_id": result.MessageID, "warning": "event not recorded"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "sent", "message_id": result.MessageID, "thread_id": result.ThreadID})
}
