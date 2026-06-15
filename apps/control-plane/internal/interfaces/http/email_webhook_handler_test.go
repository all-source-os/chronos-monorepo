package http //nolint:revive // package name intentionally matches directory

import (
	"bytes"
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

type fakeProvider struct {
	verify   bool
	msg      *emailprovider.Message
	fetchErr error
}

func (f *fakeProvider) Name() string                            { return "nylas" }
func (f *fakeProvider) VerifySignature(_ []byte, _ string) bool { return f.verify }
func (f *fakeProvider) ListThread(_ context.Context, _, _ string) (*emailprovider.Thread, error) {
	return nil, nil
}
func (f *fakeProvider) Send(_ context.Context, _ string, _ emailprovider.SendRequest) (*emailprovider.SendResult, error) {
	return nil, nil
}
func (f *fakeProvider) RegisterWebhook(_ context.Context, _ string, _ []string) (*emailprovider.WebhookRegistration, error) {
	return nil, nil
}
func (f *fakeProvider) FetchMessage(_ context.Context, _, _ string) (*emailprovider.Message, error) {
	if f.fetchErr != nil {
		return nil, f.fetchErr
	}
	return f.msg, nil
}

type fakeCore struct {
	tenant    string // GetConfig value; "" -> nil config (unknown grant)
	ingest    clients.IngestEventRequest
	ingested  bool
	ingestErr error
}

func (f *fakeCore) GetConfig(_ context.Context, key string) (*clients.ConfigEntryResponse, error) {
	if f.tenant == "" {
		return nil, nil
	}
	return &clients.ConfigEntryResponse{Key: key, Value: f.tenant}, nil
}

func (f *fakeCore) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	f.ingest = req
	f.ingested = true
	if f.ingestErr != nil {
		return nil, f.ingestErr
	}
	return &clients.IngestEventResponse{ID: "ev1"}, nil
}

func serve(h *EmailWebhookHandler, method, target string, body []byte, sig string) *httptest.ResponseRecorder {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.POST("/api/v1/webhooks/email", h.Email)
	r.GET("/api/v1/webhooks/email", h.EmailChallenge)
	req := httptest.NewRequest(method, target, bytes.NewReader(body))
	if sig != "" {
		req.Header.Set("X-Nylas-Signature", sig)
	}
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func createNote(grant, id string) []byte {
	return []byte(`{"type":"message.created","data":{"object":{"id":"` + id + `","grant_id":"` + grant + `"}}}`)
}

func sampleMessage() *emailprovider.Message {
	return &emailprovider.Message{
		ID: "msg1", ThreadID: "thr1", Subject: "Hi",
		From:    emailprovider.Address{Email: "a@b.com"},
		To:      []emailprovider.Address{{Email: "me@x.com"}},
		Snippet: "s", Body: "b", ReceivedAt: time.Unix(1750000000, 0).UTC(),
		Folder: "inbox", Labels: []string{"inbox"},
	}
}

func TestEmail_NotConfigured(t *testing.T) {
	h := NewEmailWebhookHandler(nil, &fakeCore{}, nil)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "sig")
	if w.Code != http.StatusServiceUnavailable {
		t.Fatalf("want 503, got %d", w.Code)
	}
}

func TestEmail_InvalidSignature(t *testing.T) {
	h := NewEmailWebhookHandler(&fakeProvider{verify: false}, &fakeCore{tenant: "tnt1"}, nil)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "bad")
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", w.Code)
	}
}

func TestEmail_Challenge(t *testing.T) {
	h := NewEmailWebhookHandler(&fakeProvider{verify: true}, &fakeCore{}, nil)
	w := serve(h, http.MethodGet, "/api/v1/webhooks/email?challenge=abc123", nil, "")
	if w.Code != http.StatusOK || w.Body.String() != "abc123" {
		t.Fatalf("want 200/abc123, got %d/%q", w.Code, w.Body.String())
	}
}

func TestEmail_IgnoresNonCreated(t *testing.T) {
	core := &fakeCore{tenant: "tnt1"}
	h := NewEmailWebhookHandler(&fakeProvider{verify: true}, core, nil)
	body := []byte(`{"type":"message.updated","data":{"object":{"id":"x","grant_id":"g1"}}}`)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", body, "sig")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", w.Code)
	}
	if core.ingested {
		t.Error("non-created trigger should not ingest")
	}
}

func TestEmail_UnknownGrant(t *testing.T) {
	core := &fakeCore{tenant: ""} // no config -> unknown grant
	h := NewEmailWebhookHandler(&fakeProvider{verify: true, msg: sampleMessage()}, core, nil)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "sig")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", w.Code)
	}
	if core.ingested {
		t.Error("unknown grant should not ingest")
	}
}

func TestEmail_HappyPath(t *testing.T) {
	core := &fakeCore{tenant: "tnt1"}
	h := NewEmailWebhookHandler(&fakeProvider{verify: true, msg: sampleMessage()}, core, nil)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "sig")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	if !core.ingested {
		t.Fatal("expected ingest")
	}
	got := core.ingest
	if got.EventType != "email.received" || got.EntityID != "msg1" || got.TenantID != "tnt1" {
		t.Errorf("bad envelope: %+v", got)
	}
	if got.ExpectedVersion == nil || *got.ExpectedVersion != 0 {
		t.Errorf("want first-ingest expected_version 0, got %v", got.ExpectedVersion)
	}
	if got.Metadata["provider"] != "nylas" || got.Metadata["grant_id"] != "g1" || got.Metadata["idempotency_key"] != "msg1" {
		t.Errorf("bad metadata: %+v", got.Metadata)
	}
	if got.Payload["thread_id"] != "thr1" {
		t.Errorf("bad payload: %+v", got.Payload)
	}
}

func TestEmail_DuplicateIsIdempotent(t *testing.T) {
	core := &fakeCore{tenant: "tnt1", ingestErr: clients.ErrVersionConflict}
	h := NewEmailWebhookHandler(&fakeProvider{verify: true, msg: sampleMessage()}, core, nil)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "sig")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200 on duplicate, got %d", w.Code)
	}
	if !bytes.Contains(w.Body.Bytes(), []byte("duplicate")) {
		t.Errorf("want duplicate status, got %s", w.Body.String())
	}
}

// TestEmail_SealedGrantResolves proves the P3a path: a sealed per-grant record
// in Core config is decrypted to the tenant before ingest.
func TestEmail_SealedGrantResolves(t *testing.T) {
	sealer, err := secrets.NewSealer(make([]byte, 32))
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	token, err := sealer.Seal([]byte(`{"tenant_id":"tnt-sealed","grant_id":"g1"}`))
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	core := &fakeCore{tenant: token} // GetConfig returns the sealed token as the value
	h := NewEmailWebhookHandler(&fakeProvider{verify: true, msg: sampleMessage()}, core, sealer)
	w := serve(h, http.MethodPost, "/api/v1/webhooks/email", createNote("g1", "msg1"), "sig")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	if !core.ingested {
		t.Fatal("expected ingest after sealed grant resolved")
	}
	if core.ingest.TenantID != "tnt-sealed" {
		t.Errorf("tenant not decrypted from sealed record: got %q", core.ingest.TenantID)
	}
}
