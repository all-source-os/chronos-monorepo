package usecases

import (
	"context"
	"log"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// ExpireTrialsUseCase suspends self-service trial tenants whose 14-day window
// has elapsed and who never converted to a paid plan — making the time-box
// actually bite (prompt 048, requirement 2).
//
// It is the enforcement half of the trial default: every self-service mint
// (onboard / OAuth+register / agent-register / anonymous-trial) stamps
// subscription.tier="trial" + subscription.trial_expires_at via
// TrialSubscriptionMetadata; this sweep finds those whose expiry is in the past,
// confirms they have NO active paid subscription, and SUSPENDS them.
//
// Reversible + audited, NEVER deletes:
//   - Suspend flips status to "suspended" (tenant.Suspend), the same reversible
//     state the admin Suspend/Activate tools use — an operator (or the future
//     claim/upgrade flow) can re-activate. Data is untouched.
//   - Every suspension writes a billing.trial.expired Core event and a
//     tenant.trial_expired audit event, so the action is attributable.
//
// A suspended tenant gets a clear, actionable error on access: the Query Service
// and the data-plane delegation layer reject non-active tenants up front (the
// frontend turns this into "your trial ended — pick a plan"). Suspension never
// silently no-ops the request.
type ExpireTrialsUseCase struct {
	tenantRepo repositories.TenantRepository
	auditRepo  repositories.AuditRepository
	coreClient clients.CoreClient

	// now is injected so tests get deterministic timestamps. Defaults to
	// time.Now when nil.
	now func() time.Time
}

// NewExpireTrialsUseCase wires the use case with its dependencies. coreClient may
// be nil (the billing.trial.expired Core event then becomes a no-op); the suspend
// + audit still run.
func NewExpireTrialsUseCase(
	tenantRepo repositories.TenantRepository,
	auditRepo repositories.AuditRepository,
	coreClient clients.CoreClient,
) *ExpireTrialsUseCase {
	return &ExpireTrialsUseCase{
		tenantRepo: tenantRepo,
		auditRepo:  auditRepo,
		coreClient: coreClient,
	}
}

// WithClock overrides the now-function. Tests use this for deterministic
// timestamps.
func (uc *ExpireTrialsUseCase) WithClock(now func() time.Time) *ExpireTrialsUseCase {
	uc.now = now
	return uc
}

func (uc *ExpireTrialsUseCase) currentTime() time.Time {
	if uc.now != nil {
		return uc.now()
	}
	return time.Now()
}

// ExpireTrialResult holds the outcome of evaluating one tenant.
type ExpireTrialResult struct {
	TenantID  string
	Suspended bool
	// Skipped is true when the tenant was inspected but left untouched (not a
	// trial, trial not yet expired, or already converted to a paid plan).
	Skipped bool
	Error   error
}

// ExecuteAll scans every active tenant and suspends the expired-trial,
// not-yet-paid ones. Returns one result per tenant inspected. The scheduler logs
// the aggregate counts.
func (uc *ExpireTrialsUseCase) ExecuteAll(ctx context.Context) []ExpireTrialResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("ExpireTrials: failed to list active tenants: %v", err)
		return nil
	}

	now := uc.currentTime()
	results := make([]ExpireTrialResult, 0, len(tenants))
	for _, t := range tenants {
		results = append(results, uc.evaluate(ctx, t, now))
	}
	return results
}

// evaluate decides whether a single tenant should be suspended and, if so,
// performs the reversible suspend + audit.
func (uc *ExpireTrialsUseCase) evaluate(ctx context.Context, t *entities.Tenant, now time.Time) ExpireTrialResult {
	res := ExpireTrialResult{TenantID: t.ID}

	// Only trial tenants are in scope. A tenant that upgraded to a paid tier no
	// longer reads tier=="trial", so this also covers the conversion case.
	if !TenantIsActiveTrial(t.Metadata) {
		res.Skipped = true
		return res
	}

	// A tenant that converted carries an active paid subscription in the tracked
	// subscriptions map even if the legacy subscription.tier still says "trial".
	// Belt-and-suspenders: never suspend someone who is actually paying.
	if tenantHasActivePaidSubscription(t.Metadata) {
		res.Skipped = true
		return res
	}

	expiresAt, ok := trialExpiresAt(t.Metadata)
	if !ok || !now.After(expiresAt) {
		// No parseable expiry, or the trial is still within its window.
		res.Skipped = true
		return res
	}

	// Expired trial, no paid sub → suspend (reversible).
	t.Suspend()
	if err := uc.tenantRepo.Update(t); err != nil {
		res.Error = err
		return res
	}
	res.Suspended = true

	// Core event for history (non-blocking — Core failure doesn't undo the suspend).
	if uc.coreClient != nil {
		if _, evErr := uc.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
			EventType: "billing.trial.expired",
			EntityID:  t.ID,
			Payload: map[string]any{
				"tenant_id":        t.ID,
				"trial_expires_at": expiresAt.UTC().Format(time.RFC3339),
				"suspended_at":     now.UTC().Format(time.RFC3339),
				"reason":           "trial_window_elapsed_no_paid_subscription",
			},
		}); evErr != nil {
			log.Printf("ExpireTrials: failed to write billing.trial.expired event for %s: %v", t.ID, evErr)
		}
	}

	// Audit event (non-critical), mirrors SuspendTenantUseCase.
	auditEvent, _ := entities.NewAuditEvent("tenant.trial_expired", "suspend", "SCHEDULER", "/billing/trial-expiry") //nolint:errcheck
	auditEvent.WithResource("tenant", t.ID).WithTenant(t.ID)
	auditEvent.AddMetadata("trial_expires_at", expiresAt.UTC().Format(time.RFC3339))
	_ = uc.auditRepo.Log(auditEvent) //nolint:errcheck

	return res
}

// TenantIsActiveTrial reports whether a tenant's metadata marks it as a trial
// tenant (subscription.tier == "trial", or the legacy top-level is_trial flag
// some anonymous trials set). Exported so the same trial-detection rule is shared
// with any future consumer instead of being re-derived.
func TenantIsActiveTrial(metadata map[string]interface{}) bool {
	if metadata == nil {
		return false
	}
	if sub, ok := metadata["subscription"].(map[string]interface{}); ok {
		if tier, _ := sub["tier"].(string); tier == TrialTierName { //nolint:errcheck // zero value is the intended fallback for a missing/!string tier
			return true
		}
	}
	// Legacy anonymous trials set a top-level is_trial flag in addition to the
	// subscription block; honor it so an older trial is still swept.
	if v, ok := metadata["is_trial"].(bool); ok && v {
		return true
	}
	return false
}

// trialExpiresAt reads the trial expiry timestamp from tenant metadata. It
// prefers subscription.trial_expires_at (written by TrialSubscriptionMetadata for
// every current mint) and falls back to the top-level trial_expires_at older
// anonymous trials wrote. Returns ok=false when absent or unparseable.
func trialExpiresAt(metadata map[string]interface{}) (time.Time, bool) {
	if metadata == nil {
		return time.Time{}, false
	}
	if sub, ok := metadata["subscription"].(map[string]interface{}); ok {
		if ts, ok := parseRFC3339(sub["trial_expires_at"]); ok {
			return ts, true
		}
	}
	if ts, ok := parseRFC3339(metadata["trial_expires_at"]); ok {
		return ts, true
	}
	return time.Time{}, false
}

// parseRFC3339 coerces an interface{} (string from JSON) into a time.Time.
func parseRFC3339(v interface{}) (time.Time, bool) {
	s, ok := v.(string)
	if !ok || s == "" {
		return time.Time{}, false
	}
	ts, err := time.Parse(time.RFC3339, s)
	if err != nil {
		return time.Time{}, false
	}
	return ts, true
}

// tenantHasActivePaidSubscription reports whether the tenant's tracked
// subscriptions map contains any active subscription at a PAID tier (indie and
// up — i.e. not free, not trial). Reuses entities.HighestActiveTier so the
// "what counts as active/paid" rule stays the billing module's, not a copy.
func tenantHasActivePaidSubscription(metadata map[string]interface{}) bool {
	subs := extractSubscriptionsMap(metadata)
	if len(subs) == 0 {
		return false
	}
	tier, subID := entities.HighestActiveTier(subs)
	if subID == "" {
		return false // no active subscription
	}
	switch entities.SubscriptionTier(entities.MapRetiredTier(tier)) {
	case entities.TierFree:
		return false // an "active free" sub is not a paid conversion
	default:
		return true
	}
}
