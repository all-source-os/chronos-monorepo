package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// grantProvider is the slice of the email provider the onboarding flow needs.
// *nylas.Provider satisfies it; tests provide a fake.
type grantProvider interface {
	Name() string
	AuthURL(redirectURI, state, loginHint string) (string, error)
	ExchangeCode(ctx context.Context, code, redirectURI string) (*emailprovider.Grant, error)
}

// coreConfigWriter writes a config entry to Core. clients.CoreClient satisfies it.
type coreConfigWriter interface {
	SetConfig(ctx context.Context, req clients.SetConfigRequest) error
}

const stateTTL = 10 * time.Minute

// InboxConnectHandler runs hosted-OAuth mailbox onboarding: an admin starts it
// (minting a sealed state that carries the tenant), the user consents at the
// provider, and the PUBLIC callback exchanges the code for a grant and writes a
// SEALED per-grant record to Core config (P3a invariant: no plaintext grant).
type InboxConnectHandler struct {
	provider    grantProvider
	core        coreConfigWriter
	sealer      *secrets.Sealer
	redirectURI string
	// dashboardOrigins allowlists return_to redirect targets (ADMIN_DASHBOARD_ORIGIN,
	// comma-separated) so the OAuth callback can bounce back to the admin page
	// without becoming an open redirect.
	dashboardOrigins []string
}

// NewInboxConnectHandler builds the handler. Pass a nil provider when hosted auth
// is not configured; the handler then returns 503.
func NewInboxConnectHandler(provider grantProvider, core coreConfigWriter, sealer *secrets.Sealer, redirectURI string) *InboxConnectHandler {
	return &InboxConnectHandler{
		provider:         provider,
		core:             core,
		sealer:           sealer,
		redirectURI:      redirectURI,
		dashboardOrigins: parseOrigins(os.Getenv("ADMIN_DASHBOARD_ORIGIN")),
	}
}

func parseOrigins(raw string) []string {
	var out []string
	for _, o := range strings.Split(raw, ",") {
		if o = strings.TrimRight(strings.TrimSpace(o), "/"); o != "" {
			out = append(out, o)
		}
	}
	return out
}

func (h *InboxConnectHandler) configured() bool {
	return h.provider != nil && h.core != nil && h.sealer != nil && h.redirectURI != ""
}

// allowedReturn reports whether returnTo's scheme+host is in the allowlist.
func (h *InboxConnectHandler) allowedReturn(returnTo string) bool {
	if returnTo == "" {
		return false
	}
	u, err := url.Parse(returnTo)
	if err != nil || u.Scheme == "" || u.Host == "" {
		return false
	}
	origin := u.Scheme + "://" + u.Host
	for _, allowed := range h.dashboardOrigins {
		if origin == allowed {
			return true
		}
	}
	return false
}

// connectState is sealed into the OAuth `state` param: it carries the tenant (and
// optional dashboard return target) across the redirect and authenticates the
// callback (only we hold the seal key), with an expiry to bound replay.
type connectState struct {
	TenantID string `json:"tenant_id"`
	ReturnTo string `json:"return_to,omitempty"`
	Exp      int64  `json:"exp"`
}

func (h *InboxConnectHandler) mintState(tenantID, returnTo string) (string, error) {
	b, err := json.Marshal(connectState{TenantID: tenantID, ReturnTo: returnTo, Exp: time.Now().Add(stateTTL).Unix()})
	if err != nil {
		return "", err
	}
	return h.sealer.Seal(b)
}

func (h *InboxConnectHandler) openState(token string) (*connectState, error) {
	plain, err := h.sealer.Open(token)
	if err != nil {
		return nil, err
	}
	var st connectState
	if err := json.Unmarshal(plain, &st); err != nil {
		return nil, err
	}
	if time.Now().Unix() > st.Exp {
		return nil, errors.New("state expired")
	}
	return &st, nil
}

// Start handles GET /api/v1/admin/inbox/connect (admin-gated at the route). It
// returns the hosted-auth URL the operator opens to connect a mailbox. An
// optional return_to (allowlisted) makes the callback bounce back to the dashboard.
func (h *InboxConnectHandler) Start(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "inbox onboarding not configured (needs NYLAS_CLIENT_ID, NYLAS_REDIRECT_URI, CONNECTOR_SECRET_KEY)",
		})
		return
	}
	tenantID := c.Query("tenant_id")
	if tenantID == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing 'tenant_id'"})
		return
	}
	returnTo := c.Query("return_to")
	if returnTo != "" && !h.allowedReturn(returnTo) {
		returnTo = "" // drop a non-allowlisted target rather than open-redirect
	}
	state, err := h.mintState(tenantID, returnTo)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to mint state"})
		return
	}
	authURL, err := h.provider.AuthURL(h.redirectURI, state, c.Query("email"))
	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": err.Error()})
		return
	}
	c.JSON(http.StatusOK, gin.H{"auth_url": authURL, "tenant_id": tenantID})
}

// Callback handles GET /api/v1/webhooks/inbox/connect/callback (PUBLIC — it's the
// OAuth redirect, authenticated by the sealed state rather than a JWT). On
// success it 302s back to the state's return_to when present, else returns JSON.
func (h *InboxConnectHandler) Callback(c *gin.Context) {
	if !h.configured() {
		c.JSON(http.StatusServiceUnavailable, gin.H{"error": "inbox onboarding not configured"})
		return
	}
	if oauthErr := c.Query("error"); oauthErr != "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "oauth denied: " + oauthErr})
		return
	}
	code, state := c.Query("code"), c.Query("state")
	if code == "" || state == "" {
		c.JSON(http.StatusBadRequest, gin.H{"error": "missing code or state"})
		return
	}
	st, err := h.openState(state)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid or expired state"})
		return
	}
	returnTo := ""
	if st.ReturnTo != "" && h.allowedReturn(st.ReturnTo) {
		returnTo = st.ReturnTo
	}

	grant, err := h.provider.ExchangeCode(c.Request.Context(), code, h.redirectURI)
	if err != nil {
		h.finish(c, returnTo, http.StatusBadGateway, gin.H{"status": "error", "reason": "code exchange failed"})
		return
	}

	// Seal the per-grant record before it touches Core config (P3a invariant).
	record, err := json.Marshal(map[string]any{
		"tenant_id":    st.TenantID,
		"grant_id":     grant.ID,
		"email":        grant.Email,
		"provider":     h.provider.Name(),
		"connected_at": time.Now().UTC().Format(time.RFC3339),
	})
	if err != nil {
		h.finish(c, returnTo, http.StatusInternalServerError, gin.H{"status": "error", "reason": "encode record failed"})
		return
	}
	sealed, err := h.sealer.Seal(record)
	if err != nil {
		h.finish(c, returnTo, http.StatusInternalServerError, gin.H{"status": "error", "reason": "seal failed"})
		return
	}
	if err := h.core.SetConfig(c.Request.Context(), clients.SetConfigRequest{
		Key:       grantConfigKey(grant.ID),
		Value:     sealed,
		ChangedBy: "inbox-connect",
	}); err != nil {
		h.finish(c, returnTo, http.StatusInternalServerError, gin.H{"status": "error", "reason": "failed to persist grant"})
		return
	}

	if returnTo != "" {
		q := url.Values{}
		q.Set("status", "connected")
		q.Set("grant_id", grant.ID)
		q.Set("email", grant.Email)
		c.Redirect(http.StatusFound, returnTo+"?"+q.Encode())
		return
	}
	c.JSON(http.StatusOK, gin.H{
		"status":    "connected",
		"grant_id":  grant.ID,
		"tenant_id": st.TenantID,
		"email":     grant.Email,
	})
}

// finish renders an error either as a redirect to the dashboard (?status=error)
// or as JSON when there is no return target.
func (h *InboxConnectHandler) finish(c *gin.Context, returnTo string, status int, body gin.H) {
	if returnTo != "" {
		q := url.Values{}
		q.Set("status", "error")
		if reason, ok := body["reason"].(string); ok {
			q.Set("reason", reason)
		}
		c.Redirect(http.StatusFound, returnTo+"?"+q.Encode())
		return
	}
	c.JSON(status, body)
}
