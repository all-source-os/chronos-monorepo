package usecases

import (
	"context"
	"errors"
	"time"

	"github.com/allsource/control-plane/internal/domain"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// RecoveryUseCase implements the guarded + destructive recovery actions (§4).
// It owns the guard enforcement (dry-run, confirmation, blast-radius) in the
// USE-CASE layer so the HTTP handler, the MCP, and a raw curl are all bound by
// the same rules (§6.3). Every successful mutating apply writes a Core audit
// event via the auditor BEFORE returning success.
//
// Reuse, not rebuild: reconcile_subscription delegates to the existing
// UpdateSubscriptionMetadataUseCase (the canonical billing-metadata apply path),
// and restore wraps the existing StartReplayUseCase. No new Core capability is
// added here.
type RecoveryUseCase struct {
	tenantRepo   repositories.TenantRepository
	coreClient   clients.CoreClient
	updateSubsUC *UpdateSubscriptionMetadataUseCase
	startReplay  *StartReplayUseCase
	auditor      *RecoveryAuditor
	guard        recoveryGuard
}

// RecoveryDeps bundles the RecoveryUseCase dependencies.
type RecoveryDeps struct {
	TenantRepo   repositories.TenantRepository
	CoreClient   clients.CoreClient
	UpdateSubsUC *UpdateSubscriptionMetadataUseCase
	StartReplay  *StartReplayUseCase
	Auditor      *RecoveryAuditor
	JWTSecret    string
}

// NewRecoveryUseCase creates a RecoveryUseCase.
func NewRecoveryUseCase(d RecoveryDeps) *RecoveryUseCase {
	return &RecoveryUseCase{
		tenantRepo:   d.TenantRepo,
		coreClient:   d.CoreClient,
		updateSubsUC: d.UpdateSubsUC,
		startReplay:  d.StartReplay,
		auditor:      d.Auditor,
		guard:        newRecoveryGuard(d.JWTSecret),
	}
}

// requireTenant fetches the tenant or returns ErrTenantNotFound.
func (uc *RecoveryUseCase) requireTenant(id string) (*entities.Tenant, error) {
	t, err := uc.tenantRepo.FindByID(id)
	if err != nil {
		return nil, err
	}
	return t, nil
}

// ----------------------------------------------------------------------------
// Guarded: force_resync
// ----------------------------------------------------------------------------

// ForceResync re-triggers ingestion reconciliation for a tenant. CP cannot pull
// from a tenant's upstream source itself, so the actual "resync" is: surface
// what WOULD be re-pulled (the newest event timestamp + count from the durable
// store) and, on apply, write a durable recovery marker event that the sync
// worker / operator consumes. Guarded: one tenant, reversible, dry-run preview.
func (uc *RecoveryUseCase) ForceResync(ctx context.Context, tenantID string, req RecoveryRequest) (*RecoveryResult, error) {
	const action = "force_resync"
	t, err := uc.requireTenant(tenantID)
	if err != nil {
		return nil, err
	}

	stats := uc.statsOrNil(ctx, t.ID)
	newest := uc.newestEventTime(ctx, t.ID)
	preview := map[string]any{
		"tenant_id":       t.ID,
		"event_count":     statsEventCount(stats),
		"newest_event_at": timeString(newest),
		"would_re_pull":   "events since newest_event_at from the configured sync source",
		"reversible":      true,
	}

	if req.DryRun {
		return uc.preview(ctx, action, t.ID, req, preview)
	}

	// Apply: write a durable resync-requested marker the sync worker can act on.
	if uc.coreClient != nil {
		_, _ = uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{ //nolint:errcheck // marker is best-effort; audit below is the durable record
			EventType: "tenant.resync.requested",
			EntityID:  t.ID,
			TenantID:  t.ID,
			Payload: map[string]any{
				"requested_by": req.Actor,
				"reason":       req.Reason,
				"at":           time.Now().UTC().Format(time.RFC3339Nano),
			},
		})
	}
	return uc.applied(ctx, action, t.ID, req, preview, "resync requested")
}

// ----------------------------------------------------------------------------
// Guarded: reconcile_subscription
// ----------------------------------------------------------------------------

// ReconcileSubscription re-applies QuotasForTier(effective tier) to heal quota
// drift / unmapped retired-tier aliases. Reuses the canonical
// UpdateSubscriptionMetadataUseCase apply path. Precondition: a subscription is
// present. Guarded: one tenant, dry-run shows old→new entitlements.
func (uc *RecoveryUseCase) ReconcileSubscription(ctx context.Context, tenantID string, req RecoveryRequest) (*RecoveryResult, error) {
	const action = "reconcile_subscription"
	t, err := uc.requireTenant(tenantID)
	if err != nil {
		return nil, err
	}

	sub := extractSubscriptionForHealth(t.Metadata)
	tier := effectiveBillingTier(t.Metadata, sub)
	if tier == "" {
		return nil, domain.ErrInvalidInput // no subscription to reconcile
	}
	want := entities.QuotasForTier(tier)
	have := extractQuotaForHealth(t.Metadata)

	preview := map[string]any{
		"tenant_id":      t.ID,
		"effective_tier": tier,
		"events_quota":   map[string]any{"old": have.EventsQuota, "new": want.EventsQuota},
		"queries_quota":  map[string]any{"new": want.QueriesQuota},
		"x402_allowance": map[string]any{"new": want.X402Allowance},
		"mcp_scope":      map[string]any{"new": want.MCPScope},
	}

	if req.DryRun {
		return uc.preview(ctx, action, t.ID, req, preview)
	}

	// Apply via the canonical billing-metadata path (recomputes QuotasForTier).
	if uc.updateSubsUC != nil {
		if _, err := uc.updateSubsUC.Execute(t.ID, &entities.TenantBillingMetadata{
			Subscription: &entities.SubscriptionMetadata{Tier: tier, Status: orDefault(sub.Status, "active")},
		}); err != nil {
			return nil, err
		}
	}
	return uc.applied(ctx, action, t.ID, req, preview, "subscription entitlements reconciled")
}

// ----------------------------------------------------------------------------
// Guarded: resolve_dunning
// ----------------------------------------------------------------------------

// ResolveDunning records a dunning-resolution decision for a past_due / unpaid /
// expired tenant: mark for manual review (default) or extend the grace window.
// Precondition: the tenant is actually in a dunning state. Guarded: one tenant,
// dry-run previews the action; LemonSqueezy remains the retry driver.
func (uc *RecoveryUseCase) ResolveDunning(ctx context.Context, tenantID string, req RecoveryRequest) (*RecoveryResult, error) {
	const action = "resolve_dunning"
	t, err := uc.requireTenant(tenantID)
	if err != nil {
		return nil, err
	}

	sub := extractSubscriptionForHealth(t.Metadata)
	switch normalizeDunningStatus(sub.Status) {
	case "past_due", "unpaid", "expired":
		// in dunning — proceed
	default:
		return nil, errors.Join(domain.ErrInvalidInput, errors.New("tenant is not in a dunning state"))
	}

	mode := "manual_review"
	if v, ok := req.Extra["mode"].(string); ok && v != "" {
		mode = v
	}
	preview := map[string]any{
		"tenant_id": t.ID,
		"status":    sub.Status,
		"mode":      mode, // manual_review | extend_grace | reissue_checkout
	}

	if req.DryRun {
		return uc.preview(ctx, action, t.ID, req, preview)
	}

	// Apply: record a durable dunning-decision marker. The financial action
	// (checkout reissue / grace extension) is LemonSqueezy-driven; CP records the
	// operator's decision so the billing worker / support has an event trail.
	if uc.coreClient != nil {
		_, _ = uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{ //nolint:errcheck // audit below is the durable record
			EventType: "tenant.dunning.resolution",
			EntityID:  t.ID,
			TenantID:  t.ID,
			Payload: map[string]any{
				"mode":         mode,
				"requested_by": req.Actor,
				"reason":       req.Reason,
				"at":           time.Now().UTC().Format(time.RFC3339Nano),
			},
		})
	}
	return uc.applied(ctx, action, t.ID, req, preview, "dunning resolution recorded: "+mode)
}

// ----------------------------------------------------------------------------
// shared preview / applied helpers
// ----------------------------------------------------------------------------

// preview builds a dry-run result and (for token-gated actions) attaches a
// confirm_token. tokenGated is true for actions whose apply requires an echoed
// token (rotate_keys, batch). It writes a non-fatal preview audit record.
func (uc *RecoveryUseCase) preview(ctx context.Context, action, tenantID string, req RecoveryRequest, would map[string]any) (*RecoveryResult, error) {
	res := &RecoveryResult{
		Action:   action,
		TenantID: tenantID,
		DryRun:   true,
		Would:    would,
	}
	if isTokenGated(action) {
		res.ConfirmToken = uc.guard.mintConfirmToken(action, tenantID)
	}
	// Preview audit is best-effort (no state changed) — don't fail the preview.
	_ = uc.auditor.Record(ctx, RecoveryAuditRecord{ //nolint:errcheck
		Action: action, TenantID: tenantID, Actor: req.Actor,
		DryRun: true, Reason: req.Reason, Outcome: "preview", Details: would,
	})
	return res, nil
}

// applied writes the MANDATORY audit event, then returns the apply result. If
// the audit write fails the apply is reported as an error (the audit must be
// durable before success — §6.3 rule 1).
func (uc *RecoveryUseCase) applied(ctx context.Context, action, tenantID string, req RecoveryRequest, details map[string]any, msg string) (*RecoveryResult, error) {
	if err := uc.auditor.Record(ctx, RecoveryAuditRecord{
		Action: action, TenantID: tenantID, Actor: req.Actor,
		DryRun: false, Reason: req.Reason, Outcome: "applied", Details: details,
	}); err != nil {
		return nil, err
	}
	return &RecoveryResult{
		Action:   action,
		TenantID: tenantID,
		DryRun:   false,
		Applied:  true,
		Message:  msg,
	}, nil
}

// --- small helpers ---

func (uc *RecoveryUseCase) statsOrNil(ctx context.Context, id string) *clients.TenantStatsResponse {
	if uc.coreClient == nil {
		return nil
	}
	s, err := uc.coreClient.GetTenantStats(ctx, id)
	if err != nil {
		return nil
	}
	return s
}

func (uc *RecoveryUseCase) newestEventTime(ctx context.Context, tenantID string) time.Time {
	if uc.coreClient == nil {
		return time.Time{}
	}
	resp, err := uc.coreClient.QueryEvents(ctx, clients.QueryEventsRequest{
		TenantID: tenantID, Limit: 1, Order: "desc",
	})
	if err != nil || resp == nil || len(resp.Events) == 0 {
		return time.Time{}
	}
	ts, _ := timeFromEvent(resp.Events[0].Timestamp)
	return ts
}

func statsEventCount(s *clients.TenantStatsResponse) int64 {
	if s == nil {
		return 0
	}
	return s.EventCount
}

func timeString(t time.Time) string {
	if t.IsZero() {
		return ""
	}
	return t.UTC().Format(time.RFC3339)
}

func timeFromEvent(ts string) (time.Time, bool) {
	if t, err := time.Parse(time.RFC3339Nano, ts); err == nil {
		return t, true
	}
	if t, err := time.Parse(time.RFC3339, ts); err == nil {
		return t, true
	}
	return time.Time{}, false
}

func orDefault(s, def string) string {
	if s == "" {
		return def
	}
	return s
}

func normalizeDunningStatus(s string) string {
	out := make([]byte, 0, len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		if c == ' ' {
			continue
		}
		out = append(out, c)
	}
	return string(out)
}

// isTokenGated reports whether an action's apply requires an echoed confirm_token
// minted by a prior dry-run (rotate_keys, batch). reprovision/restore use
// typed-name confirm_tenant_id instead, so they are NOT token-gated here.
func isTokenGated(action string) bool {
	switch action {
	case "rotate_keys", "batch":
		return true
	default:
		return false
	}
}
