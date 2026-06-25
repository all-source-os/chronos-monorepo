package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"net/http"
	"testing"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"

	"github.com/allsource/control-plane/internal/application/usecases"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/persistence"
)

// viewAsTestJWTSecret is the shared HMAC secret for the view-as handler tests. It
// is reused as both the admin-token signing key (AdminAuthMiddleware) and the
// view-as token signing key so a minted token validates with the same secret a
// real CP would use.
const viewAsTestJWTSecret = commsTestJWTSecret

// testViewAsClaims is a local mirror of the main-package Claims used ONLY to decode
// a minted view-as token in the test and assert its shape (the http package can't
// import the main package). It carries exactly the fields the contract (§5.1 /
// prompt 041) pins: sub, tenant_id, role, view_as, act_as, exp, iat.
type testViewAsClaims struct {
	Sub      string `json:"sub"`
	TenantID string `json:"tenant_id"`
	Role     string `json:"role"`
	ViewAs   bool   `json:"view_as"`
	ActAs    string `json:"act_as"`
	jwt.StandardClaims
}

// testViewAsSigner mints a readonly+view_as ~15m token off the test secret,
// mirroring AuthClient.SignViewAsJWT (which lives in the main package and can't be
// imported here). It uses usecases.ViewAsTokenTTL so the test asserts the CANONICAL
// TTL the production signer also uses.
func testViewAsSigner(adminUserID, targetTenantID string) (string, error) {
	now := time.Now()
	claims := &testViewAsClaims{
		Sub:      adminUserID,
		TenantID: targetTenantID,
		Role:     string(entities.RoleReadOnly),
		ViewAs:   true,
		ActAs:    adminUserID,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: now.Add(usecases.ViewAsTokenTTL).Unix(),
			IssuedAt:  now.Unix(),
			Issuer:    "allsource",
			Subject:   adminUserID,
		},
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(viewAsTestJWTSecret))
}

type viewAsTestEnv struct {
	repo *persistence.MemoryTenantRepository
	core *commsMockCore
	uc   *usecases.ViewAsUseCase
	h    *ViewAsHandler
}

func newViewAsTestEnv(t *testing.T) *viewAsTestEnv {
	t.Helper()
	repo := persistence.NewMemoryTenantRepository()
	core := &commsMockCore{}
	auditor := usecases.NewViewAsAuditor(core)
	uc := usecases.NewViewAsUseCase(usecases.ViewAsDeps{
		TenantRepo: repo,
		Signer:     testViewAsSigner,
		Auditor:    auditor,
	})
	return &viewAsTestEnv{repo: repo, core: core, uc: uc, h: NewViewAsHandler(uc)}
}

// router builds the real admin router (AdminAuthMiddleware) with the view-as
// routes, so the admin-role gate is exercised exactly as in production.
func (e *viewAsTestEnv) router() *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	admin := r.Group("/api/v1/admin")
	admin.Use(AdminAuthMiddleware(viewAsTestJWTSecret))
	admin.POST("/tenants/:id/view-as", e.h.Start)
	admin.POST("/tenants/:id/view-as/stop", e.h.Stop)
	return r
}

// viewAsEvents returns the events recorded under the admin-viewas system tenant,
// optionally filtered to one event type.
func (e *viewAsTestEnv) viewAsEvents(eventType string) []map[string]any {
	out := []map[string]any{}
	for _, ev := range e.core.ingested {
		if ev.TenantID != usecases.ViewAsAuditTenant {
			continue
		}
		if eventType != "" && ev.EventType != eventType {
			continue
		}
		out = append(out, map[string]any{"type": ev.EventType, "entity": ev.EntityID, "payload": ev.Payload})
	}
	return out
}

// ============================================================================
// Mint: returns a readonly + view_as, ~15m token DISTINCT from any session
// (Phase 7 acceptance).
// ============================================================================

func TestViewAs_StartMintsScopedReadonlyToken(t *testing.T) {
	env := newViewAsTestEnv(t)
	seedTenant(t, env.repo, "t-9", "Tenant Nine", nil)
	r := env.router()
	adminTok := commsAdminToken(t) // sub=admin-1, role=admin

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-9/view-as", adminTok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("view-as start: expected 200, got %d (%s)", w.Code, w.Body.String())
	}

	var res usecases.ViewAsStartResult
	mustJSON(t, w, &res)

	// The result advertises a readonly + view_as token with the canonical TTL.
	if res.Role != "readonly" {
		t.Fatalf("expected role readonly, got %q", res.Role)
	}
	if !res.ViewAs {
		t.Fatalf("expected view_as=true in result")
	}
	if res.TenantID != "t-9" || res.TenantName != "Tenant Nine" {
		t.Fatalf("expected tenant t-9/Tenant Nine, got %q/%q", res.TenantID, res.TenantName)
	}
	if res.TTLSeconds != int64(usecases.ViewAsTokenTTL.Seconds()) {
		t.Fatalf("expected ttl %d sec, got %d", int64(usecases.ViewAsTokenTTL.Seconds()), res.TTLSeconds)
	}
	if res.Token == "" {
		t.Fatalf("expected a non-empty token")
	}

	// The token is DISTINCT from the admin session token.
	if res.Token == adminTok {
		t.Fatalf("view-as token must be distinct from the admin session token")
	}

	// Decode the minted token and assert the contract shape (sub=admin, role
	// readonly, view_as true, act_as=admin, tenant_id=viewed, short exp).
	parsed := &testViewAsClaims{}
	tok, err := jwt.ParseWithClaims(res.Token, parsed, func(*jwt.Token) (interface{}, error) {
		return []byte(viewAsTestJWTSecret), nil
	})
	if err != nil || !tok.Valid {
		t.Fatalf("minted token failed to parse/validate: %v", err)
	}
	if parsed.Sub != "admin-1" {
		t.Fatalf("expected sub=admin-1 (the impersonating admin), got %q", parsed.Sub)
	}
	if parsed.ActAs != "admin-1" {
		t.Fatalf("expected act_as=admin-1 (the real actor), got %q", parsed.ActAs)
	}
	if parsed.TenantID != "t-9" {
		t.Fatalf("expected tenant_id=t-9 (the viewed tenant), got %q", parsed.TenantID)
	}
	if parsed.Role != string(entities.RoleReadOnly) {
		t.Fatalf("expected role readonly, got %q", parsed.Role)
	}
	if !parsed.ViewAs {
		t.Fatalf("expected view_as=true claim")
	}

	// Short TTL: exp within (now, now+15m+slack]. A long-lived token is a defect.
	ttl := time.Until(time.Unix(parsed.ExpiresAt, 0))
	if ttl <= 0 || ttl > usecases.ViewAsTokenTTL+time.Minute {
		t.Fatalf("expected a short ~15m TTL, got %s (exp=%d)", ttl, parsed.ExpiresAt)
	}

	// Mint wrote admin.viewas.started.
	started := env.viewAsEvents(usecases.ViewAsStartedEventType)
	if len(started) != 1 {
		t.Fatalf("expected exactly one admin.viewas.started event, got %d", len(started))
	}
	if started[0]["entity"] != "viewas:t-9" {
		t.Fatalf("expected started entity viewas:t-9, got %v", started[0]["entity"])
	}
}

// ============================================================================
// Admin gate: 403 without an admin JWT (Phase 7 acceptance).
// ============================================================================

func TestViewAs_StartRequiresAdminJWT(t *testing.T) {
	env := newViewAsTestEnv(t)
	seedTenant(t, env.repo, "t-9", "Tenant Nine", nil)
	r := env.router()

	// (a) No token at all → 401 (unauthorized).
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-9/view-as", "", nil)
	if w.Code != http.StatusUnauthorized {
		t.Fatalf("no token: expected 401, got %d (%s)", w.Code, w.Body.String())
	}

	// (b) A valid NON-admin (readonly) JWT → 403 (forbidden by AdminAuthMiddleware).
	// This is also exactly what a leaked view_as token (role:readonly) would get if
	// pointed at the mint endpoint — it can never re-mint.
	nonAdmin := signRoleToken(t, "readonly-user", entities.RoleReadOnly)
	w = doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-9/view-as", nonAdmin, nil)
	if w.Code != http.StatusForbidden {
		t.Fatalf("non-admin token: expected 403, got %d (%s)", w.Code, w.Body.String())
	}

	// No token was minted and no started event was written on the rejected calls.
	if got := env.viewAsEvents(usecases.ViewAsStartedEventType); len(got) != 0 {
		t.Fatalf("expected no started events on rejected mints, got %d", len(got))
	}
}

// ============================================================================
// Pairing: every started has a paired stopped (Phase 7 acceptance).
// ============================================================================

func TestViewAs_StartStopAuditPairs(t *testing.T) {
	env := newViewAsTestEnv(t)
	seedTenant(t, env.repo, "t-9", "Tenant Nine", nil)
	r := env.router()
	adminTok := commsAdminToken(t)

	// Start → admin.viewas.started.
	w := doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-9/view-as", adminTok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("start: expected 200, got %d (%s)", w.Code, w.Body.String())
	}

	// Stop → admin.viewas.stopped (reason defaults to exit).
	w = doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/t-9/view-as/stop", adminTok, nil)
	if w.Code != http.StatusOK {
		t.Fatalf("stop: expected 200, got %d (%s)", w.Code, w.Body.String())
	}

	started := env.viewAsEvents(usecases.ViewAsStartedEventType)
	stopped := env.viewAsEvents(usecases.ViewAsStoppedEventType)
	if len(started) != 1 {
		t.Fatalf("expected 1 started, got %d", len(started))
	}
	if len(stopped) != 1 {
		t.Fatalf("expected 1 stopped (paired with the start), got %d", len(stopped))
	}
	// Both events bracket the SAME entity (viewas:<tenant>) so the audit shows a
	// clean start→stop pair.
	if started[0]["entity"] != stopped[0]["entity"] {
		t.Fatalf("start/stop entities differ: %v vs %v", started[0]["entity"], stopped[0]["entity"])
	}
	if got, _ := stopped[0]["payload"].(map[string]any)["reason"].(string); got != usecases.ViewAsStopReasonExit {
		t.Fatalf("expected stop reason %q, got %q", usecases.ViewAsStopReasonExit, got)
	}
}

// ============================================================================
// Mint refuses a non-existent tenant (never impersonate a non-tenant).
// ============================================================================

func TestViewAs_StartUnknownTenant404(t *testing.T) {
	env := newViewAsTestEnv(t)
	r := env.router()
	adminTok := commsAdminToken(t)

	w := doReq(t, r, http.MethodPost, "/api/v1/admin/tenants/nope/view-as", adminTok, nil)
	if w.Code != http.StatusNotFound {
		t.Fatalf("unknown tenant: expected 404, got %d (%s)", w.Code, w.Body.String())
	}
	if got := env.viewAsEvents(usecases.ViewAsStartedEventType); len(got) != 0 {
		t.Fatalf("expected no started event for an unknown tenant, got %d", len(got))
	}
}

// signRoleToken mints an AdminClaims-shaped JWT with an arbitrary role, used to
// prove the admin gate rejects non-admin roles (e.g. a readonly/view_as token).
func signRoleToken(t *testing.T, sub string, role entities.Role) string {
	t.Helper()
	claims := &AdminClaims{
		UserID:   sub,
		Role:     role,
		TenantID: "t-9",
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: time.Now().Add(time.Hour).Unix(),
			IssuedAt:  time.Now().Unix(),
		},
	}
	s, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(viewAsTestJWTSecret))
	if err != nil {
		t.Fatalf("sign role token: %v", err)
	}
	return s
}

// compile-time assurance the signer matches the use-case's expected type.
var _ usecases.ViewAsSignerFunc = testViewAsSigner

// silence unused import in case the file is trimmed; context is used by the mock.
var _ = context.Background
