package usecases

import (
	"context"
	"errors"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// ViewAsTokenTTL is the short time-box on a "view as tenant" impersonation token
// (ADMIN_TENANT_POWER_TOOL §5.1). A support session is minutes, so 15m bounds the
// blast radius if the token leaks and is the auto-expiry backstop for a forgotten
// "Exit". It is a tiny fraction of the 7-day real session — view-as must NEVER be
// a standing credential. This is the CANONICAL value: AuthClient.SignViewAsJWT
// references it so the minted token's exp and the use case's reported expiry can
// never drift.
const ViewAsTokenTTL = 15 * time.Minute

// ViewAsSignerFunc mints a read-only, short-TTL "view as tenant" impersonation
// token for the given admin user viewing the given tenant. It is satisfied by
// AuthClient.SignViewAsJWT — injected (not imported) so the use case stays in the
// application layer and the signing key never leaves AuthClient.
type ViewAsSignerFunc func(adminUserID, targetTenantID string) (string, error)

// ErrViewAsUnavailable is returned when no signer is wired (the CP cannot mint a
// view-as token — a misconfiguration, surfaced as a 503 by the handler).
var ErrViewAsUnavailable = errors.New("view-as: token signer not configured")

// ViewAsUseCase implements the read-only "view as tenant" impersonation flow
// (ADMIN_TENANT_POWER_TOOL §5). It mints a scoped, short-TTL token DISTINCT from
// the tenant's real session and audits every start/stop as a durable Core event.
//
// Read-only BY CONSTRUCTION — this use case only MINTS the token and records the
// audit; it has no path that writes on the tenant's behalf. The token it returns
// carries role:readonly + view_as:true, which the role check and the
// ViewAsWriteRefusal middleware enforce independently (§5.2).
type ViewAsUseCase struct {
	tenantRepo repositories.TenantRepository
	sign       ViewAsSignerFunc
	auditor    *ViewAsAuditor
}

// ViewAsDeps bundles the ViewAsUseCase dependencies.
type ViewAsDeps struct {
	TenantRepo repositories.TenantRepository
	// Signer mints the view-as token (AuthClient.SignViewAsJWT). nil → Start
	// reports ErrViewAsUnavailable.
	Signer ViewAsSignerFunc
	// Auditor writes admin.viewas.* events. nil-safe (test mode → no-op).
	Auditor *ViewAsAuditor
}

// NewViewAsUseCase creates a ViewAsUseCase.
func NewViewAsUseCase(d ViewAsDeps) *ViewAsUseCase {
	return &ViewAsUseCase{
		tenantRepo: d.TenantRepo,
		sign:       d.Signer,
		auditor:    d.Auditor,
	}
}

// ViewAsStartResult is the wire response from minting a view-as token (matches
// what prompt 041, the admin frame, consumes to set the scoped viewas_token
// cookie and render the banner/countdown).
type ViewAsStartResult struct {
	// Token is the read-only, view_as, short-TTL JWT. DISTINCT from the admin's
	// session and from the tenant's real session.
	Token string `json:"token"`
	// TenantID is the viewed tenant (echoed for the banner).
	TenantID string `json:"tenant_id"`
	// TenantName is the viewed tenant's display name (for the banner copy).
	TenantName string `json:"tenant_name,omitempty"`
	// Role is always "readonly" — surfaced so the client can assert it.
	Role string `json:"role"`
	// ViewAs is always true — the impersonation marker.
	ViewAs bool `json:"view_as"`
	// ExpiresAt is the token's Unix expiry; the banner shows the countdown from it.
	ExpiresAt int64 `json:"expires_at"`
	// TTLSeconds is the time-box length (ViewAsTokenTTL) for convenience.
	TTLSeconds int64 `json:"ttl_seconds"`
}

// Start mints a view-as token for targetTenantID and records admin.viewas.started
// BEFORE returning it (so a started event always precedes any read with the token).
// actor is the real admin user id from the JWT (never client-trusted). The tenant
// must exist (404 otherwise) so we never mint a token for a non-tenant.
func (uc *ViewAsUseCase) Start(ctx context.Context, targetTenantID, actor string) (*ViewAsStartResult, error) {
	if targetTenantID == "" {
		return nil, domain.ErrInvalidInput
	}
	if uc.sign == nil {
		return nil, ErrViewAsUnavailable
	}

	// Resolve the tenant so we (a) refuse to mint for a non-existent tenant and
	// (b) can put its name on the banner. ErrTenantNotFound bubbles to a 404.
	tenant, err := uc.tenantRepo.FindByID(targetTenantID)
	if err != nil {
		return nil, err
	}

	token, err := uc.sign(actor, targetTenantID)
	if err != nil {
		return nil, err
	}

	exp := time.Now().Add(ViewAsTokenTTL).Unix()

	// Audit the start BEFORE returning the token. A failed audit fails the mint —
	// the start MUST be durable so the start/stop pairing can never be incomplete.
	if auditErr := uc.auditor.RecordStarted(ctx, targetTenantID, actor, exp); auditErr != nil {
		return nil, auditErr
	}

	name := ""
	if tenant != nil {
		name = tenant.Name
	}
	return &ViewAsStartResult{
		Token:      token,
		TenantID:   targetTenantID,
		TenantName: name,
		Role:       "readonly",
		ViewAs:     true,
		ExpiresAt:  exp,
		TTLSeconds: int64(ViewAsTokenTTL.Seconds()),
	}, nil
}

// Stop records admin.viewas.stopped for targetTenantID. It is called by the
// explicit Exit endpoint (reason=exit) and by the lazy-expiry path (reason=expired)
// so every started has a paired stopped in the audit. There is no token to revoke
// (the JWT is stateless + short-TTL); Stop is purely the audit teardown. actor is
// the admin behind the view (from the admin JWT on the Exit call, or from the
// expiring view_as token's act_as on the expiry path).
func (uc *ViewAsUseCase) Stop(ctx context.Context, targetTenantID, actor, reason string) error {
	if targetTenantID == "" {
		return domain.ErrInvalidInput
	}
	if reason == "" {
		reason = ViewAsStopReasonExit
	}
	return uc.auditor.RecordStopped(ctx, targetTenantID, actor, reason)
}
