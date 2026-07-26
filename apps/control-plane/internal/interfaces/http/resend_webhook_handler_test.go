package http //nolint:revive // package name intentionally matches directory

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
)

type fakeResendProvider struct {
	verify   bool
	msg      *emailprovider.Message
	fetchErr error
}

func (f *fakeResendProvider) Name() string                               { return "resend" }
func (f *fakeResendProvider) VerifyWebhook(_ http.Header, _ []byte) bool { return f.verify }
func (f *fakeResendProvider) FetchMessage(_ context.Context, _, _ string) (*emailprovider.Message, error) {
	if f.fetchErr != nil {
		return nil, f.fetchErr
	}
	return f.msg, nil
}

func serveResend(h *ResendWebhookHandler, body []byte) *httptest.ResponseRecorder {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/api/v1/webhooks/resend/inbound", h.Inbound)
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/webhooks/resend/inbound", bytes.NewReader(body))
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

// sealedAddr seals a Resend connection record for addr so the fakeCore's
// GetConfig returns it and resolveTenant can open it.
func sealedAddr(t *testing.T, tenant, addr string) string {
	t.Helper()
	rec, _ := json.Marshal(grantRecord{TenantID: tenant, GrantID: addr, Email: addr, Provider: "resend"})
	token, err := newSealer(t).Seal(rec)
	if err != nil {
		t.Fatalf("seal: %v", err)
	}
	return token
}

func resendBody(emailID string, to ...string) []byte {
	b, _ := json.Marshal(map[string]any{
		"type": "email.received",
		"data": map[string]any{"email_id": emailID, "to": to},
	})
	return b
}

func TestResend_InboundIngests(t *testing.T) {
	sealer := newSealer(t)
	// Re-seal with the SAME sealer the handler uses (newSealer is deterministic per
	// the test key), so resolveTenant can open it.
	rec, _ := json.Marshal(grantRecord{TenantID: "t1", GrantID: "sales@x.com", Email: "sales@x.com", Provider: "resend"})
	token, _ := sealer.Seal(rec)
	core := &fakeCore{tenant: token} // GetConfig returns the sealed record for any key
	provider := &fakeResendProvider{verify: true, msg: sampleMessage()}
	h := NewResendWebhookHandler(provider, core, sealer)

	w := serveResend(h, resendBody("em1", "sales@x.com"))
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d: %s", w.Code, w.Body.String())
	}
	if !core.ingested {
		t.Fatal("expected an email.received ingest")
	}
	if core.ingest.EventType != "email.received" || core.ingest.EntityID != "msg1" {
		t.Errorf("bad ingest: type=%s entity=%s", core.ingest.EventType, core.ingest.EntityID)
	}
	if core.ingest.TenantID != "t1" {
		t.Errorf("tenant should resolve from the To address, got %q", core.ingest.TenantID)
	}
	if core.ingest.Payload["grant_id"] != "sales@x.com" {
		t.Errorf("payload grant_id should scope to the mailbox, got %v", core.ingest.Payload["grant_id"])
	}
}

func TestResend_InboundBadSignature(t *testing.T) {
	h := NewResendWebhookHandler(&fakeResendProvider{verify: false}, &fakeCore{}, newSealer(t))
	if got := serveResend(h, resendBody("em1", "sales@x.com")).Code; got != http.StatusBadRequest {
		t.Fatalf("want 400 on bad signature, got %d", got)
	}
}

func TestResend_InboundUnknownRecipient(t *testing.T) {
	// core.tenant "" → GetConfig returns nil → no registered address → ignored.
	h := NewResendWebhookHandler(&fakeResendProvider{verify: true, msg: sampleMessage()}, &fakeCore{}, newSealer(t))
	w := serveResend(h, resendBody("em1", "nobody@x.com"))
	if w.Code != http.StatusOK || !bytes.Contains(w.Body.Bytes(), []byte("ignored")) {
		t.Fatalf("want 200 ignored for unknown recipient, got %d: %s", w.Code, w.Body.String())
	}
}

func TestResend_InboundIgnoresNonReceived(t *testing.T) {
	h := NewResendWebhookHandler(&fakeResendProvider{verify: true}, &fakeCore{tenant: sealedAddr(t, "t1", "sales@x.com")}, newSealer(t))
	body, _ := json.Marshal(map[string]any{"type": "email.delivered", "data": map[string]any{}})
	w := serveResend(h, body)
	if w.Code != http.StatusOK || !bytes.Contains(w.Body.Bytes(), []byte("ignored")) {
		t.Fatalf("want 200 ignored for non-received type, got %d: %s", w.Code, w.Body.String())
	}
}

// ── Engagement webhook (prompt 050) ────────────────────────────────────────

// fakeEngagement records RecordEngagement calls and simulates the recorder's
// idempotency: a repeated (message_id, type) returns "duplicate".
type fakeEngagement struct {
	calls int
	seen  map[string]bool
}

func (f *fakeEngagement) RecordEngagement(_ context.Context, espMessageID, espEventType, _, _ string) (string, error) {
	f.calls++
	if f.seen == nil {
		f.seen = map[string]bool{}
	}
	key := espMessageID + ":" + espEventType
	if f.seen[key] {
		return "duplicate", nil
	}
	f.seen[key] = true
	return "ingested", nil
}

func serveResendEvents(h *ResendWebhookHandler, body []byte) *httptest.ResponseRecorder {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/api/v1/webhooks/resend/events", h.Events)
	req := httptest.NewRequestWithContext(context.Background(), http.MethodPost, "/api/v1/webhooks/resend/events", bytes.NewReader(body))
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func engagementBody(eventType, emailID, link string) []byte {
	data := map[string]any{"email_id": emailID}
	if link != "" {
		data["click"] = map[string]any{"link": link}
	}
	b, _ := json.Marshal(map[string]any{"type": eventType, "created_at": "2026-06-01T00:00:00Z", "data": data})
	return b
}

func TestResend_EngagementIngestThenDuplicate(t *testing.T) {
	eng := &fakeEngagement{}
	h := NewResendWebhookHandler(&fakeResendProvider{verify: true}, &fakeCore{}, newSealer(t)).WithEngagement(eng)

	w1 := serveResendEvents(h, engagementBody("email.clicked", "em-1", "https://x"))
	if w1.Code != http.StatusOK || !bytes.Contains(w1.Body.Bytes(), []byte("ingested")) {
		t.Fatalf("first engagement: want 200 ingested, got %d: %s", w1.Code, w1.Body.String())
	}
	// Replay the SAME webhook → exactly one logical event (status duplicate).
	w2 := serveResendEvents(h, engagementBody("email.clicked", "em-1", "https://x"))
	if w2.Code != http.StatusOK || !bytes.Contains(w2.Body.Bytes(), []byte("duplicate")) {
		t.Fatalf("replayed engagement: want 200 duplicate, got %d: %s", w2.Code, w2.Body.String())
	}
}

func TestResend_EngagementBadSignature(t *testing.T) {
	eng := &fakeEngagement{}
	h := NewResendWebhookHandler(&fakeResendProvider{verify: false}, &fakeCore{}, newSealer(t)).WithEngagement(eng)
	if got := serveResendEvents(h, engagementBody("email.delivered", "em-1", "")).Code; got != http.StatusBadRequest {
		t.Fatalf("want 400 on bad signature, got %d", got)
	}
	if eng.calls != 0 {
		t.Errorf("a bad-signature webhook must NOT record engagement, got %d calls", eng.calls)
	}
}

func TestResend_EngagementNotConfigured(t *testing.T) {
	// No engagement recorder wired → 503 (inbound path unaffected).
	h := NewResendWebhookHandler(&fakeResendProvider{verify: true}, &fakeCore{}, newSealer(t))
	if got := serveResendEvents(h, engagementBody("email.delivered", "em-1", "")).Code; got != http.StatusServiceUnavailable {
		t.Fatalf("want 503 when engagement not configured, got %d", got)
	}
}

func TestResend_EngagementMissingEmailID(t *testing.T) {
	h := NewResendWebhookHandler(&fakeResendProvider{verify: true}, &fakeCore{}, newSealer(t)).WithEngagement(&fakeEngagement{})
	if got := serveResendEvents(h, engagementBody("email.delivered", "", "")).Code; got != http.StatusBadRequest {
		t.Fatalf("want 400 on missing email_id, got %d", got)
	}
}
