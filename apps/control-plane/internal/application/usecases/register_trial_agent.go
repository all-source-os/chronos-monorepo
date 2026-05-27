package usecases

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"log"
	"time"

	"github.com/google/uuid"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Trial-tier quota constants. Smaller than the standard agent free tier
// because the trial is unauthenticated and meant to fund a try-it-out
// path, not a sustained workload. Per the §"Open Questions" of
// docs/proposals/AGENT_DRIVEN_PRIME_ONBOARDING.md these defaults can be
// retuned without a wire-format change.
const (
	TrialEventsQuota   = 1000
	TrialQueriesQuota  = 100
	TrialValidDuration = 14 * 24 * time.Hour // 14 days

	// Tier label written into tenant metadata so authorization and billing
	// can distinguish trial tenants from real ones. Lowercase, snake_case.
	TrialTierName = "trial"
)

// RegisterTrialAgentUseCase mints an anonymous trial tenant + API key
// without requiring prior authentication. Distinct from
// RegisterAgentUseCase because:
//   - Tenant ID is random, not derived from a user-supplied name
//   - The tenant carries `is_trial=true`, `trial_expires_at`, and a
//     `claim_token` in metadata so a real user can later claim it via
//     POST /api/v1/agents/claim (separate follow-up bead)
//   - Lower quotas (see Trial*Quota constants above)
//   - Always invoked under IP-level rate limiting at the route layer
//
// Closes the MINT half of neotoma-driven Gap 1 (bead t-072c). The CLAIM
// half lives in a follow-up bead.
type RegisterTrialAgentUseCase struct {
	createTenantUC *CreateTenantUseCase
	auditRepo      repositories.AuditRepository
	coreClient     clients.CoreClient
	signKey        KeySignerFunc

	// claimBaseURL is prepended to the claim_token when building claim_url.
	// Defaults to "https://www.all-source.xyz/connect?claim=" when empty,
	// matching the production /connect deep-link contract (bead t-baff).
	// Injected so tests can override without environment coupling.
	claimBaseURL string

	// now is injected so tests get deterministic timestamps. Defaults to
	// time.Now when nil.
	now func() time.Time
}

// NewRegisterTrialAgentUseCase wires the use case with its dependencies.
func NewRegisterTrialAgentUseCase(
	createTenantUC *CreateTenantUseCase,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
	signKey KeySignerFunc,
) *RegisterTrialAgentUseCase {
	return &RegisterTrialAgentUseCase{
		createTenantUC: createTenantUC,
		auditRepo:      auditRepo,
		coreClient:     coreClient,
		signKey:        signKey,
	}
}

// WithClaimBaseURL overrides the URL prefix used in claim_url. Tests use
// this; in production the default is fine.
func (uc *RegisterTrialAgentUseCase) WithClaimBaseURL(url string) *RegisterTrialAgentUseCase {
	uc.claimBaseURL = url
	return uc
}

// WithClock overrides the now-function. Tests use this for deterministic
// timestamps.
func (uc *RegisterTrialAgentUseCase) WithClock(now func() time.Time) *RegisterTrialAgentUseCase {
	uc.now = now
	return uc
}

// Execute creates an anonymous trial tenant, mints an API key, writes an
// audit event, and returns the wire response. Idempotency is not required
// — each call produces a fresh tenant; rate limiting at the route layer
// prevents abuse.
func (uc *RegisterTrialAgentUseCase) Execute(
	ctx context.Context,
	req dto.RegisterTrialAgentRequest,
) (*dto.RegisterTrialAgentResponse, error) {
	now := uc.currentTime()
	expiresAt := now.Add(TrialValidDuration)

	// Random tenant ID — keep it short so it's readable in logs but
	// collision-resistant. 16 hex chars = 64 bits of entropy.
	tenantSuffix, err := randomHex(8)
	if err != nil {
		return nil, fmt.Errorf("generate tenant suffix: %w", err)
	}
	tenantID := "trial-" + tenantSuffix

	// Single-use migration token. UUIDv4 — cryptographically random and
	// the format real users will see in claim URLs.
	claimToken := uuid.NewString()
	claimURL := uc.buildClaimURL(claimToken)

	agentName := req.AgentName
	if agentName == "" {
		agentName = "anonymous-trial-" + tenantSuffix
	}

	// Create tenant. Metadata carries every trial-specific signal so
	// downstream consumers (auth, billing, the future claim endpoint) can
	// branch on `is_trial` without re-querying.
	tenantResp, err := uc.createTenantUC.Execute(dto.CreateTenantRequest{
		ID:          tenantID,
		Name:        agentName,
		Description: "Anonymous trial tenant (created via /api/v1/agents/anonymous-trial)",
		Metadata: map[string]interface{}{
			"is_trial":           true,
			"trial_expires_at":   expiresAt.UTC().Format(time.RFC3339),
			"claim_token":        claimToken,
			"client_fingerprint": req.ClientFingerprint,
			"subscription": map[string]interface{}{
				"tier":   TrialTierName,
				"status": "active",
			},
			"quota": map[string]interface{}{
				"events_quota":  TrialEventsQuota,
				"queries_quota": TrialQueriesQuota,
			},
		},
	})
	if err != nil {
		return nil, err
	}

	// Sign API key. Same JWT machinery as RegisterAgentUseCase — trial
	// differentiation lives in tenant metadata, not the key prefix.
	// (Prefix-based differentiation would require changing every call
	// site of SignAPIKey, which is far out of scope for this bead.
	// Documented as a known divergence from the proposal in the bead
	// close note.)
	apiKey, err := uc.signKey(tenantResp.ID, agentName, entities.RoleServiceAccount)
	if err != nil {
		return nil, fmt.Errorf("sign API key: %w", err)
	}

	// Audit + event log — non-blocking, mirrors RegisterAgentUseCase.
	auditEvent, _ := entities.NewAuditEvent("agent.trial_registered", "create", "POST", "/agents/anonymous-trial") //nolint:errcheck
	auditEvent.WithResource("agent", tenantID).WithTenant(tenantID)
	auditEvent.AddMetadata("is_trial", "true")
	auditEvent.AddMetadata("client_fingerprint", req.ClientFingerprint)
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	uc.writeTrialRegisteredEvent(ctx, tenantID, req, expiresAt, claimToken)

	return &dto.RegisterTrialAgentResponse{
		TenantID:   tenantResp.ID,
		APIKey:     apiKey,
		Tier:       TrialTierName,
		ExpiresAt:  expiresAt.UTC().Format(time.RFC3339),
		ClaimToken: claimToken,
		ClaimURL:   claimURL,
		Quotas: dto.AgentQuotas{
			EventsQuota:  TrialEventsQuota,
			QueriesQuota: TrialQueriesQuota,
		},
	}, nil
}

// currentTime returns the use case's clock; defaults to time.Now.
func (uc *RegisterTrialAgentUseCase) currentTime() time.Time {
	if uc.now != nil {
		return uc.now()
	}
	return time.Now()
}

// buildClaimURL returns the claim URL the human visits after signing in
// to migrate the trial tenant into their real account.
func (uc *RegisterTrialAgentUseCase) buildClaimURL(token string) string {
	base := uc.claimBaseURL
	if base == "" {
		base = "https://www.all-source.xyz/connect?claim="
	}
	return base + token
}

// writeTrialRegisteredEvent emits agent.trial_registered to Core for the
// audit trail. Non-blocking — Core failure does not fail the mint.
func (uc *RegisterTrialAgentUseCase) writeTrialRegisteredEvent(
	ctx context.Context,
	tenantID string,
	req dto.RegisterTrialAgentRequest,
	expiresAt time.Time,
	claimToken string,
) {
	if uc.coreClient == nil {
		return
	}
	_, err := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: "agent.trial_registered",
		EntityID:  tenantID,
		Payload: map[string]any{
			"tier":               TrialTierName,
			"expires_at":         expiresAt.UTC().Format(time.RFC3339),
			"claim_token":        claimToken,
			"client_fingerprint": req.ClientFingerprint,
			"events_quota":       TrialEventsQuota,
			"queries_quota":      TrialQueriesQuota,
		},
	})
	if err != nil {
		log.Printf("[agent-trial] failed to write agent.trial_registered event for %s: %v", tenantID, err)
	}
}

// randomHex returns a cryptographically-random lowercase hex string of
// length 2*n. Used for tenant ID suffixes — short, URL-safe, no special
// chars that need escaping in audit logs or filesystem paths.
func randomHex(n int) (string, error) {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}
