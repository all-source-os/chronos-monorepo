package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"net/http"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/clients/llm"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// inboxCore is the slice of the Core client the admin inbox endpoints need.
// clients.CoreClient satisfies it; tests provide a fake.
type inboxCore interface {
	ListConfigs(ctx context.Context) (*clients.ListConfigsResponse, error)
	SetConfig(ctx context.Context, req clients.SetConfigRequest) error
	DeleteConfig(ctx context.Context, key string) error
	IngestEvent(ctx context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error)
	QueryEvents(ctx context.Context, req clients.QueryEventsRequest) (*clients.QueryEventsResponse, error)
}

// inboxSender is the slice of the email provider the send endpoint needs.
// *resend.Provider satisfies it.
type inboxSender interface {
	Name() string
	Send(ctx context.Context, grantID string, req emailprovider.SendRequest) (*emailprovider.SendResult, error)
}

// InboxDrafter generates AI reply drafts grounded in thread + contact context
// (045). *llm.Client satisfies it; nil disables the generate endpoint (503).
type InboxDrafter interface {
	GenerateReply(ctx context.Context, system, user string) (string, error)
}

// InboxAdminHandler exposes the AI inbox over admin HTTP so a dashboard can
// manage it: list/disconnect connections (sealed grants), read the email.*
// message stream from Core, and triage/draft/send (the HTTP twins of the
// prime-mcp inbox verbs — epic P1). All routes are admin-gated.
type InboxAdminHandler struct {
	core    inboxCore
	sealer  *secrets.Sealer
	sender  inboxSender  // may be nil when no provider is configured
	drafter InboxDrafter // may be nil when no LLM is configured (045)
}

// NewInboxAdminHandler builds the handler. core/sealer nil → endpoints 503.
func NewInboxAdminHandler(core inboxCore, sealer *secrets.Sealer, sender inboxSender) *InboxAdminHandler {
	return &InboxAdminHandler{core: core, sealer: sealer, sender: sender}
}

// WithDrafter attaches an AI draft generator (045) and returns the handler for
// chaining. A nil drafter leaves generation disabled (GenerateDraft → 503), so
// the rest of the inbox works unchanged when ANTHROPIC_API_KEY is unset.
func (h *InboxAdminHandler) WithDrafter(d InboxDrafter) *InboxAdminHandler {
	h.drafter = d
	return h
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

// AddAddress handles POST /api/v1/admin/inbox/addresses — registers a receiving
// address (e.g. sales@all-source.xyz) to a tenant for providers without an OAuth
// grant flow (Resend: a connection is just a verified address). Seals
// {tenant_id, grant_id=address, email, provider} under the address so the inbound
// webhook resolves the tenant and the stream scopes to the mailbox. grant_id ≡
// the address; send uses it as the From.
func (h *InboxAdminHandler) AddAddress(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	var req struct {
		TenantID string `json:"tenant_id"`
		Email    string `json:"email"`
	}
	addr := ""
	if c.ShouldBindJSON(&req) == nil {
		addr = strings.ToLower(strings.TrimSpace(req.Email))
	}
	if req.TenantID == "" || addr == "" || !strings.Contains(addr, "@") {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id and a valid email are required"})
		return
	}
	provider := "resend"
	if h.sender != nil {
		provider = h.sender.Name()
	}
	record, err := json.Marshal(grantRecord{
		TenantID:    req.TenantID,
		GrantID:     addr,
		Email:       addr,
		Provider:    provider,
		ConnectedAt: time.Now().UTC().Format(time.RFC3339),
	})
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "encode record failed"})
		return
	}
	sealed, err := h.sealer.Seal(record)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "seal failed"})
		return
	}
	if err := h.core.SetConfig(c.Request.Context(), clients.SetConfigRequest{
		Key:       grantConfigKey(addr),
		Value:     sealed,
		ChangedBy: "inbox-add-address",
	}); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to persist address"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "connected", "grant_id": addr, "email": addr, "tenant_id": req.TenantID})
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
	// Optionally scope to a single mailbox (grant) within the tenant — Core
	// filters on payload, and every email.* event carries payload.grant_id.
	payloadFilter := ""
	if grantID := c.Query("grant_id"); grantID != "" {
		if b, err := json.Marshal(map[string]string{"grant_id": grantID}); err == nil {
			payloadFilter = string(b)
		}
	}
	res, err := h.core.QueryEvents(c.Request.Context(), clients.QueryEventsRequest{
		EventTypePrefix: "email.", // matches email.received/sent/triaged/… (exact event_type would match nothing)
		TenantID:        tenantID,
		PayloadFilter:   payloadFilter,
		Order:           "desc",
		Limit:           limit,
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
		GrantID   string `json:"grant_id"`
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
	if req.GrantID != "" {
		payload["grant_id"] = req.GrantID // scope this event to the mailbox (stream filter)
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
		GrantID   string                  `json:"grant_id"`
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
	if req.GrantID != "" {
		payload["grant_id"] = req.GrantID // scope this event to the mailbox (stream filter)
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
		GrantID   string                  `json:"grant_id"` // which connected mailbox to send from
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
	// Send from the specified mailbox; fall back to the tenant's only/first one.
	grantID := conns[0].GrantID
	if req.GrantID != "" {
		grantID = ""
		for _, cn := range conns {
			if cn.GrantID == req.GrantID {
				grantID = cn.GrantID
				break
			}
		}
		if grantID == "" {
			c.JSON(http.StatusNotFound, gin.H{"error": "that mailbox is not connected for the tenant"})
			return
		}
	}

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
			"grant_id":  grantID, // scope to the mailbox (stream filter)
		},
		Metadata: meta,
	}); err != nil {
		// The mail was sent; failing to record the event is logged, not fatal to the caller.
		c.JSON(http.StatusOK, gin.H{"status": "sent", "message_id": result.MessageID, "warning": "event not recorded"})
		return
	}
	c.JSON(http.StatusOK, gin.H{"status": "sent", "message_id": result.MessageID, "thread_id": result.ThreadID})
}

// --- AI draft generation (045) ---

// GenerateDraft handles POST /api/v1/admin/inbox/draft/generate — it drafts a
// reply with the LLM, grounded in (1) the thread's inbound messages and (2) the
// contact's prior threads, both read from Core's durable email.received events
// (Core is the source of truth — no fresh provider fetch). It does NOT write an
// event: generation is a suggestion the human edits; Draft/Send then write the
// durable email.drafted / email.sent events.
func (h *InboxAdminHandler) GenerateDraft(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox not configured"})
		return
	}
	if h.drafter == nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "draft generation not configured (ANTHROPIC_API_KEY unset)"})
		return
	}
	var req struct {
		TenantID     string `json:"tenant_id"`
		ThreadID     string `json:"thread_id"`
		GrantID      string `json:"grant_id"`
		Intent       string `json:"intent"`
		Tone         string `json:"tone"`
		MailboxEmail string `json:"mailbox_email"`
	}
	if c.ShouldBindJSON(&req) != nil || req.TenantID == "" || req.ThreadID == "" || strings.TrimSpace(req.Intent) == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "tenant_id, thread_id and intent are required"})
		return
	}

	ctx := c.Request.Context()
	// Scope to the mailbox (grant) when known, else the whole tenant. Pull the
	// mailbox's recent received mail once, then partition it into the thread and
	// the contact's other threads (the recall).
	payloadFilter := ""
	if req.GrantID != "" {
		if b, err := json.Marshal(map[string]string{"grant_id": req.GrantID}); err == nil {
			payloadFilter = string(b)
		}
	}
	res, err := h.core.QueryEvents(ctx, clients.QueryEventsRequest{
		EventType:     "email.received", // exact: only inbound carries from/subject/body
		TenantID:      req.TenantID,
		PayloadFilter: payloadFilter,
		Order:         "desc",
		Limit:         100,
	})
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{"error": "failed to load thread context"})
		return
	}

	thread, recall := splitThreadAndRecall(res.Events, req.ThreadID)
	system, user := buildDraftPrompt(req.MailboxEmail, req.Intent, req.Tone, thread, recall)

	body, err := h.drafter.GenerateReply(ctx, system, user)
	switch {
	case errors.Is(err, llm.ErrRefused):
		c.JSON(http.StatusUnprocessableEntity, gin.H{"error": "the model declined to draft this reply"})
		return
	case err != nil:
		c.JSON(http.StatusBadGateway, gin.H{"error": "draft generation failed"})
		return
	}

	c.JSON(http.StatusOK, gin.H{
		"body":      body,
		"thread_id": req.ThreadID,
		"grant_id":  req.GrantID,
		"grounded_on": gin.H{
			"thread_messages": len(thread),
			"prior_threads":   countThreads(recall),
		},
	})
}

// mailMsg is a received message reconstructed from a Core email.received event,
// used to ground draft generation.
type mailMsg struct {
	threadID string
	from     string // sender email, lowercased
	fromName string
	subject  string
	snippet  string
	body     string
	at       string // event timestamp
}

// pstr reads a string field from a generic payload map (missing/typed-nil → "").
func pstr(m map[string]any, key string) string {
	if m == nil {
		return ""
	}
	if v, ok := m[key].(string); ok {
		return v
	}
	return ""
}

// extractMail pulls the grounding fields out of an email.received payload (the
// provider-neutral contract; `from` is a {name,email} object).
func extractMail(e clients.EventEntry) mailMsg {
	m := mailMsg{
		threadID: pstr(e.Payload, "thread_id"),
		subject:  pstr(e.Payload, "subject"),
		snippet:  pstr(e.Payload, "snippet"),
		body:     pstr(e.Payload, "body"),
		at:       e.Timestamp,
	}
	if from, ok := e.Payload["from"].(map[string]any); ok {
		m.from = strings.ToLower(strings.TrimSpace(pstr(from, "email")))
		m.fromName = pstr(from, "name")
	}
	return m
}

// splitThreadAndRecall partitions received events (newest-first from Core) into
// the target thread's messages (re-ordered oldest-first as a transcript) and the
// contact's other recent threads (the recall). The contact is the sender of the
// thread's most recent inbound message; recall is capped at 5 distinct threads.
func splitThreadAndRecall(events []clients.EventEntry, threadID string) (thread, recall []mailMsg) {
	for _, e := range events {
		if m := extractMail(e); m.threadID == threadID {
			thread = append(thread, m)
		}
	}
	sort.Slice(thread, func(i, j int) bool { return thread[i].at < thread[j].at })
	if len(thread) == 0 {
		return thread, nil
	}
	contact := thread[len(thread)-1].from
	if contact == "" {
		return thread, nil
	}
	seen := map[string]bool{threadID: true}
	for _, e := range events { // newest-first: first hit per thread is its latest message
		m := extractMail(e)
		if m.from != contact || seen[m.threadID] {
			continue
		}
		seen[m.threadID] = true
		recall = append(recall, m)
		if len(recall) >= 5 {
			break
		}
	}
	return thread, recall
}

func countThreads(msgs []mailMsg) int {
	seen := map[string]bool{}
	for _, m := range msgs {
		seen[m.threadID] = true
	}
	return len(seen)
}

const (
	maxBodyChars    = 1500
	maxSnippetChars = 240
)

func trimText(s string, n int) string {
	s = strings.TrimSpace(s)
	if len(s) > n {
		return strings.TrimSpace(s[:n]) + "…"
	}
	return s
}

// buildDraftPrompt assembles the grounded prompt: a system frame (reply-only,
// no fabrication, tone-match) and a user message carrying the thread transcript,
// the contact's prior-thread context, and the operator's intent.
func buildDraftPrompt(mailbox, intent, tone string, thread, recall []mailMsg) (system, user string) {
	who := "this mailbox"
	if mailbox != "" {
		who = mailbox
	}
	var s strings.Builder
	s.WriteString("You are an expert email assistant drafting a reply on behalf of " + who + ".\n")
	s.WriteString("Write ONLY the reply body — no subject line, no preamble like \"Here's a draft\", no placeholder tokens like [Name]. ")
	s.WriteString("Match the tone, formality, and language of the thread. Be concise and specific. ")
	s.WriteString("Do NOT invent facts, commitments, prices, dates, or names that aren't supported by the thread or the operator's instruction; if information is missing, write around it rather than fabricating.")
	if t := strings.TrimSpace(tone); t != "" {
		s.WriteString(" Use a " + t + " tone.")
	}
	system = s.String()

	var u strings.Builder
	u.WriteString("You are replying within the email thread below. The most recent message is last.\n\n")
	u.WriteString("=== THREAD ===\n")
	if len(thread) == 0 {
		u.WriteString("(No prior messages were found for this thread.)\n")
	}
	for _, m := range thread {
		name := m.fromName
		if name == "" {
			name = m.from
		}
		u.WriteString("From: " + name + " <" + m.from + ">")
		if m.at != "" {
			u.WriteString("  (" + m.at + ")")
		}
		u.WriteString("\n")
		if m.subject != "" {
			u.WriteString("Subject: " + m.subject + "\n")
		}
		content := m.body
		if strings.TrimSpace(content) == "" {
			content = m.snippet
		}
		u.WriteString(trimText(content, maxBodyChars) + "\n---\n")
	}

	u.WriteString("\n=== PRIOR CONTEXT WITH THIS CONTACT (for grounding; do not quote verbatim) ===\n")
	if len(recall) == 0 {
		u.WriteString("No prior threads with this contact.\n")
	}
	for _, m := range recall {
		subj := m.subject
		if subj == "" {
			subj = "(no subject)"
		}
		line := "- "
		if m.at != "" {
			line += "(" + m.at + ") "
		}
		line += subj
		if snip := trimText(m.snippet, maxSnippetChars); snip != "" {
			line += " — " + snip
		}
		u.WriteString(line + "\n")
	}

	u.WriteString("\n=== YOUR TASK ===\nWrite the reply. Operator's intent for this reply: " + strings.TrimSpace(intent) + "\n")
	return system, u.String()
}
