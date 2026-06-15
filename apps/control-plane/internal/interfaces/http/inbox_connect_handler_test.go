package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

type fakeGrantProvider struct {
	grant *emailprovider.Grant
	exErr error
}

func (f *fakeGrantProvider) Name() string { return "nylas" }
func (f *fakeGrantProvider) AuthURL(redirectURI, state, loginHint string) (string, error) {
	return "https://auth.example/v3/connect/auth?redirect_uri=" + redirectURI + "&state=" + state, nil
}
func (f *fakeGrantProvider) ExchangeCode(_ context.Context, _, _ string) (*emailprovider.Grant, error) {
	if f.exErr != nil {
		return nil, f.exErr
	}
	return f.grant, nil
}

type fakeConfigWriter struct {
	set    clients.SetConfigRequest
	called bool
	err    error
}

func (f *fakeConfigWriter) SetConfig(_ context.Context, req clients.SetConfigRequest) error {
	f.set = req
	f.called = true
	return f.err
}

func newSealer(t *testing.T) *secrets.Sealer {
	t.Helper()
	s, err := secrets.NewSealer(make([]byte, 32))
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	return s
}

func serveConnect(h *InboxConnectHandler, target string) *httptest.ResponseRecorder {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.GET("/api/v1/admin/inbox/connect", h.Start)
	r.GET("/api/v1/webhooks/inbox/connect/callback", h.Callback)
	req := httptest.NewRequest(http.MethodGet, target, nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)
	return w
}

func TestConnect_StartNotConfigured(t *testing.T) {
	h := NewInboxConnectHandler(nil, &fakeConfigWriter{}, newSealer(t), "") // no provider, no redirect
	w := serveConnect(h, "/api/v1/admin/inbox/connect?tenant_id=tnt1")
	if w.Code != http.StatusServiceUnavailable {
		t.Fatalf("want 503, got %d", w.Code)
	}
}

func TestConnect_StartReturnsAuthURL(t *testing.T) {
	h := NewInboxConnectHandler(&fakeGrantProvider{}, &fakeConfigWriter{}, newSealer(t), "https://api.all-source.xyz/cb")
	w := serveConnect(h, "/api/v1/admin/inbox/connect?tenant_id=tnt1&email=sales@all-source.xyz")
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	var resp struct {
		AuthURL  string `json:"auth_url"`
		TenantID string `json:"tenant_id"`
	}
	if err := json.Unmarshal(w.Body.Bytes(), &resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp.TenantID != "tnt1" || !strings.Contains(resp.AuthURL, "state=") {
		t.Errorf("bad start response: %+v", resp)
	}
}

func TestConnect_StartMissingTenant(t *testing.T) {
	h := NewInboxConnectHandler(&fakeGrantProvider{}, &fakeConfigWriter{}, newSealer(t), "https://api.all-source.xyz/cb")
	w := serveConnect(h, "/api/v1/admin/inbox/connect")
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", w.Code)
	}
}

func TestConnect_CallbackPersistsSealedGrant(t *testing.T) {
	sealer := newSealer(t)
	core := &fakeConfigWriter{}
	provider := &fakeGrantProvider{grant: &emailprovider.Grant{ID: "grant_xyz", Email: "sales@all-source.xyz", Provider: "google"}}
	h := NewInboxConnectHandler(provider, core, sealer, "https://api.all-source.xyz/cb")

	state, err := h.mintState("tnt1")
	if err != nil {
		t.Fatalf("mintState: %v", err)
	}
	w := serveConnect(h, "/api/v1/webhooks/inbox/connect/callback?code=abc&state="+url.QueryEscape(state))
	if w.Code != http.StatusOK {
		t.Fatalf("want 200, got %d (%s)", w.Code, w.Body.String())
	}
	if !core.called {
		t.Fatal("expected SetConfig")
	}
	if core.set.Key != grantConfigKey("grant_xyz") {
		t.Errorf("bad config key: %q", core.set.Key)
	}
	sealed, ok := core.set.Value.(string)
	if !ok || !secrets.IsSealed(sealed) {
		t.Fatalf("config value is not a sealed string: %v", core.set.Value)
	}
	// The sealed record must decrypt back to the tenant.
	plain, err := sealer.Open(sealed)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	var rec struct {
		TenantID string `json:"tenant_id"`
		GrantID  string `json:"grant_id"`
	}
	if json.Unmarshal(plain, &rec) != nil || rec.TenantID != "tnt1" || rec.GrantID != "grant_xyz" {
		t.Errorf("bad sealed record: %s", plain)
	}
}

func TestConnect_CallbackRejectsInvalidState(t *testing.T) {
	h := NewInboxConnectHandler(&fakeGrantProvider{grant: &emailprovider.Grant{ID: "g"}}, &fakeConfigWriter{}, newSealer(t), "https://api.all-source.xyz/cb")
	w := serveConnect(h, "/api/v1/webhooks/inbox/connect/callback?code=abc&state=garbage")
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", w.Code)
	}
}

func TestConnect_CallbackRejectsExpiredState(t *testing.T) {
	sealer := newSealer(t)
	core := &fakeConfigWriter{}
	h := NewInboxConnectHandler(&fakeGrantProvider{grant: &emailprovider.Grant{ID: "g"}}, core, sealer, "https://api.all-source.xyz/cb")
	// Seal a state that already expired.
	expired, _ := json.Marshal(connectState{TenantID: "tnt1", Exp: time.Now().Add(-time.Minute).Unix()})
	token, _ := sealer.Seal(expired)
	w := serveConnect(h, "/api/v1/webhooks/inbox/connect/callback?code=abc&state="+url.QueryEscape(token))
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400 for expired state, got %d", w.Code)
	}
	if core.called {
		t.Error("expired state must not write config")
	}
}

func TestConnect_CallbackOAuthDenied(t *testing.T) {
	h := NewInboxConnectHandler(&fakeGrantProvider{}, &fakeConfigWriter{}, newSealer(t), "https://api.all-source.xyz/cb")
	w := serveConnect(h, "/api/v1/webhooks/inbox/connect/callback?error=access_denied")
	if w.Code != http.StatusBadRequest {
		t.Fatalf("want 400, got %d", w.Code)
	}
}
