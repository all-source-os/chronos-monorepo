package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
)

type fakeInboxCore struct {
	configs  []clients.ConfigEntryResponse
	events   []clients.EventEntry
	deleted  string
	ingested []clients.IngestEventRequest
	err      error
}

func (f *fakeInboxCore) ListConfigs(_ context.Context) (*clients.ListConfigsResponse, error) {
	if f.err != nil {
		return nil, f.err
	}
	return &clients.ListConfigsResponse{Configs: f.configs}, nil
}
func (f *fakeInboxCore) DeleteConfig(_ context.Context, key string) error {
	f.deleted = key
	return f.err
}
func (f *fakeInboxCore) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	f.ingested = append(f.ingested, req)
	if f.err != nil {
		return nil, f.err
	}
	return &clients.IngestEventResponse{ID: "ev1"}, nil
}
func (f *fakeInboxCore) QueryEvents(_ context.Context, _ clients.QueryEventsRequest) (*clients.QueryEventsResponse, error) {
	if f.err != nil {
		return nil, f.err
	}
	return &clients.QueryEventsResponse{Events: f.events, Count: len(f.events)}, nil
}

type fakeSender struct {
	result *emailprovider.SendResult
	called bool
	err    error
}

func (f *fakeSender) Name() string { return "nylas" }
func (f *fakeSender) Send(_ context.Context, _ string, _ emailprovider.SendRequest) (*emailprovider.SendResult, error) {
	f.called = true
	if f.err != nil {
		return nil, f.err
	}
	return f.result, nil
}

func serveInbox(h *InboxAdminHandler, method, target, body string) *httptest.ResponseRecorder {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.GET("/api/v1/admin/inbox/connections", h.ListConnections)
	r.DELETE("/api/v1/admin/inbox/connections/:grant_id", h.Disconnect)
	r.GET("/api/v1/admin/inbox/messages", h.Messages)
	r.POST("/api/v1/admin/inbox/triage", h.Triage)
	r.POST("/api/v1/admin/inbox/draft", h.Draft)
	r.POST("/api/v1/admin/inbox/send", h.Send)
	var rdr *strings.Reader
	if body != "" {
		rdr = strings.NewReader(body)
	} else {
		rdr = strings.NewReader("")
	}
	req := httptest.NewRequest(method, target, rdr)
	if body != "" {
		req.Header.Set("Content-Type", "application/json")
	}
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func TestInbox_NotConfigured(t *testing.T) {
	h := NewInboxAdminHandler(nil, nil, nil)
	if serveInbox(h, http.MethodGet, "/api/v1/admin/inbox/connections", "").Code != http.StatusServiceUnavailable {
		t.Fatal("want 503 when not configured")
	}
}

func TestInbox_ListConnectionsDecrypts(t *testing.T) {
	sealer := newSealer(t)
	rec, _ := json.Marshal(grantRecord{TenantID: "t1", GrantID: "g1", Email: "sales@x.com", Provider: "nylas"})
	token, _ := sealer.Seal(rec)
	core := &fakeInboxCore{configs: []clients.ConfigEntryResponse{
		{Key: "connector:email:grant:g1", Value: token},
		{Key: "unrelated:config", Value: "plain"},
	}}
	h := NewInboxAdminHandler(core, sealer, nil)
	w := serveInbox(h, http.MethodGet, "/api/v1/admin/inbox/connections", "")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", w.Code)
	}
	var resp struct {
		Connections []grantRecord `json:"connections"`
		Count       int           `json:"count"`
	}
	_ = json.Unmarshal(w.Body.Bytes(), &resp)
	if resp.Count != 1 || resp.Connections[0].Email != "sales@x.com" || resp.Connections[0].TenantID != "t1" {
		t.Errorf("bad connections: %+v", resp)
	}
}

func TestInbox_Disconnect(t *testing.T) {
	core := &fakeInboxCore{}
	h := NewInboxAdminHandler(core, newSealer(t), nil)
	w := serveInbox(h, http.MethodDelete, "/api/v1/admin/inbox/connections/g1", "")
	if w.Code != http.StatusOK || core.deleted != "connector:email:grant:g1" {
		t.Fatalf("bad disconnect: code=%d deleted=%q", w.Code, core.deleted)
	}
}

func TestInbox_Messages(t *testing.T) {
	core := &fakeInboxCore{events: []clients.EventEntry{{EventType: "email.received", EntityID: "m1"}}}
	h := NewInboxAdminHandler(core, newSealer(t), nil)
	if serveInbox(h, http.MethodGet, "/api/v1/admin/inbox/messages", "").Code != http.StatusBadRequest {
		t.Fatal("want 400 without tenant_id")
	}
	w := serveInbox(h, http.MethodGet, "/api/v1/admin/inbox/messages?tenant_id=t1", "")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d", w.Code)
	}
}

func TestInbox_Triage(t *testing.T) {
	core := &fakeInboxCore{}
	h := NewInboxAdminHandler(core, newSealer(t), nil)
	if serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/triage", `{"tenant_id":"t1","message_id":"m1","label":"bogus"}`).Code != http.StatusBadRequest {
		t.Fatal("want 400 for bad label")
	}
	w := serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/triage", `{"tenant_id":"t1","message_id":"m1","label":"needs-reply"}`)
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	if len(core.ingested) != 1 || core.ingested[0].EventType != "email.triaged" || core.ingested[0].EntityID != "m1" {
		t.Errorf("bad triage event: %+v", core.ingested)
	}
	if core.ingested[0].Payload["label"] != "needs-reply" {
		t.Errorf("bad triage payload: %+v", core.ingested[0].Payload)
	}
}

func TestInbox_Draft(t *testing.T) {
	core := &fakeInboxCore{}
	h := NewInboxAdminHandler(core, newSealer(t), nil)
	if serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/draft", `{"tenant_id":"t1","thread_id":"th1","intent":"x"}`).Code != http.StatusBadRequest {
		t.Fatal("want 400 without body")
	}
	w := serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/draft", `{"tenant_id":"t1","thread_id":"th1","body":"hi","intent":"reply"}`)
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	var resp struct {
		DraftID string `json:"draft_id"`
	}
	_ = json.Unmarshal(w.Body.Bytes(), &resp)
	if !strings.HasPrefix(resp.DraftID, "draft_") {
		t.Errorf("bad draft_id: %q", resp.DraftID)
	}
	if core.ingested[0].EventType != "email.drafted" || core.ingested[0].Metadata["draft_id"] != resp.DraftID {
		t.Errorf("bad draft event: %+v", core.ingested[0])
	}
}

func TestInbox_SendConfirmGatedAndRecords(t *testing.T) {
	sealer := newSealer(t)
	rec, _ := json.Marshal(grantRecord{TenantID: "t1", GrantID: "g1", Email: "sales@x.com", Provider: "nylas"})
	token, _ := sealer.Seal(rec)
	core := &fakeInboxCore{configs: []clients.ConfigEntryResponse{{Key: "connector:email:grant:g1", Value: token}}}
	sender := &fakeSender{result: &emailprovider.SendResult{MessageID: "ms1", ThreadID: "th1", SentAt: time.Now()}}
	h := NewInboxAdminHandler(core, sealer, sender)

	// no confirm -> 400, nothing sent
	if serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/send", `{"tenant_id":"t1","to":[{"email":"a@b.com"}],"body":"hi"}`).Code != http.StatusBadRequest {
		t.Fatal("want 400 without confirm")
	}
	if sender.called {
		t.Fatal("must not send without confirm")
	}

	// confirm -> 200, sent + email.sent recorded
	w := serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/send", `{"tenant_id":"t1","to":[{"email":"a@b.com"}],"body":"hi","confirm":true}`)
	if w.Code != http.StatusOK || !sender.called {
		t.Fatalf("want 200 + sent, got %d called=%v (%s)", w.Code, sender.called, w.Body.String())
	}
	var sent *clients.IngestEventRequest
	for i := range core.ingested {
		if core.ingested[i].EventType == "email.sent" {
			sent = &core.ingested[i]
		}
	}
	if sent == nil || sent.EntityID != "ms1" || sent.Metadata["grant_id"] != "g1" {
		t.Errorf("bad email.sent: %+v", core.ingested)
	}
}

func TestInbox_SendNoConnection(t *testing.T) {
	core := &fakeInboxCore{} // no connections
	sender := &fakeSender{result: &emailprovider.SendResult{MessageID: "x"}}
	h := NewInboxAdminHandler(core, newSealer(t), sender)
	w := serveInbox(h, http.MethodPost, "/api/v1/admin/inbox/send", `{"tenant_id":"t1","to":[{"email":"a@b.com"}],"body":"hi","confirm":true}`)
	if w.Code != http.StatusNotFound {
		t.Fatalf("want 404 when tenant has no inbox, got %d", w.Code)
	}
}
