package usecases

import (
	"context"
	"fmt"
	"regexp"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/allsource/control-plane/internal/application/dto"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// AnalyzeTenants is a READ-ONLY fleet-data analysis pass. It scans every tenant
// (or one, or one category) and returns anomaly findings so the admin console can
// surface data problems instead of an operator eyeballing 40+ rows.
//
// READ-ONLY CONTRACT: this use case and its handler perform ZERO mutations — no
// POST/PUT/DELETE to Core, no repository writes. Mutation already has guarded
// homes (reap-demo, backfill-usage reconcile, recovery/*); every Finding instead
// carries a SuggestedAction that DEEP-LINKS to one of those existing routes. A
// reviewer can confirm no writes by grepping this file for the Core client's
// write methods (CreateTenant/DeleteTenant/UpdateTenant*/SetConfig/IngestEvent
// /UpdateConfig/DeleteConfig) — there are none.
//
// REUSE, not reinvent (so the analysis and the real pages can never disagree):
//   - counts: extractEventCount/extractMemberCount (list_tenants.go) + the same
//     Core GetTenantStats cross-check the admin list uses (degrades to the
//     metadata mirror on any Core miss).
//   - plan: extractPlan (list_tenants.go) + extractSubscriptionForHealth /
//     effectiveBillingTier (fleet_health_helpers.go) + the canonical tier set
//     from entities.subscription (single tier authority).
//   - health: FleetHealthUseCase.ComputeTenant — the SAME 11-signal assessment
//     /fleet/health serves; at_risk/critical roll into a usage_health finding.
//   - plan-vs-billing (fleet): VerifyBillingConfigUseCase.Execute — the SAME
//     report /billing/config-check serves.

// --- Stable finding codes (API CONTRACT — the admin UI switches on these). ---
//
// Each constant is the `code` field of a Finding. Names are stable; changing one
// is a breaking change for the UI (prompt 047). Grouped by category.
const (
	// data_integrity
	CodeCountCapArtifact = "count_cap_artifact"  // event_count == exactly the backfill cap (1,000,000)
	CodeUnmeteredOrEmpty = "unmetered_or_empty"  // 0 metered events for a tenant older than the grace window
	CodeCreatedAtNotReal = "created_at_not_real" // FLEET: >=8 tenants share one created_at calendar date (time.Now()-on-load smell)
	CodeMissingFields    = "missing_required_fields"
	CodePlanShapeDrift   = "plan_shape_drift" // tenant `plan` metadata is not a clean string (DTO shape-drift class)

	// plan_billing
	CodePlanNotInCatalog        = "plan_not_in_catalog"          // tier not in {free,indie,studio,scale,enterprise}
	CodePaidPlanNoSubscription  = "paid_plan_no_subscription"    // paid tier but no active subscription metadata
	CodePlanDistributionSuspect = "plan_distribution_suspicious" // FLEET: every tenant collapses to one plan / free carries paid entitlements
	CodeBillingConfigBroken     = "billing_config_broken"        // FLEET: VerifyBillingConfig reports error-severity issues

	// litter
	CodeDemoLitter        = "demo_litter"          // Core flagged is_demo
	CodeTrialOrTestTenant = "trial_or_test_tenant" // name matches a trial/test pattern
	CodeDuplicateName     = "duplicate_name"       // duplicate / near-duplicate tenant name
	CodeEmptyTenant       = "empty_tenant"         // 0 events AND 0 members, older than the grace window

	// usage_health
	CodeQuotaPressure = "quota_pressure" // events_used/quota >= 0.9 (>=1.0 critical)
	CodeHealthAtRisk  = "health_at_risk" // FleetHealth tier at_risk/critical
	CodeStaleTenant   = "stale_tenant"   // no recent events (recency signal)

	// cross-cutting
	CodeAnalysisDegraded = "analysis_degraded" // a sub-check for THIS tenant failed; result is partial, run not failed
)

// Analysis tuning constants (named once; match ADMIN_HEALTH.md definitions).
const (
	// backfillCountCap is the exact value backfill-usage writes when it hits its
	// page cap (defaultBackfillMaxPages × queryPageLimit = 2000 × 500). An
	// event_count of EXACTLY this is the cap artifact, not a real count
	// (ADMIN_HEALTH.md "What number does the admin show?"). The uncapped 5-min
	// SyncEventsUsageUseCase reconciler supersedes it.
	backfillCountCap int64 = 1_000_000

	// tenantGraceWindow is the age below which a 0-count / empty tenant is "too
	// new to judge" — a freshly-created tenant legitimately has no events yet.
	tenantGraceWindow = 24 * time.Hour

	// createdAtSharedDateThreshold is the §data-integrity-gate rule: >= this many
	// tenants sharing ONE created_at calendar date is the time.Now()-on-load smell
	// (ADMIN_HEALTH.md gate "Fake created_at": ">=8 tenants share ONE date").
	createdAtSharedDateThreshold = 8

	// createdAtDominanceFraction guards against a false positive proven 2026-06-26:
	// the time.Now()-on-load bug collapses EVERY tenant onto one date, so the
	// dominant bucket covers nearly the whole fleet. A LEGITIMATE same-day batch
	// (e.g. an e2e run seeding 15 test tenants in one minute — names whose epoch
	// suffix decodes to that very day) is a minority among many real distinct
	// dates and must NOT be flagged as fake. Only fire when the worst bucket
	// dominates the fleet (or there is effectively just one distinct date).
	createdAtDominanceFraction = 0.7

	// quotaPressureWarn/Critical are the usage_health thresholds (prompt 046 §4).
	quotaPressureWarn     = 0.9
	quotaPressureCritical = 1.0

	// analysisMaxConcurrency caps the per-tenant Core round-trips so an on-demand
	// fleet scan doesn't fan out unbounded.
	analysisMaxConcurrency = 8
)

// canonicalTiers is the set of tiers a tenant plan is ALLOWED to be in (after
// MapRetiredTier normalizes retired aliases). Sourced from entities.subscription
// so this can never disagree with the tier authority — same set verify_billing
// and signals.PaidTier use.
var canonicalTiers = map[string]bool{
	string(entities.TierFree):       true,
	string(entities.TierIndie):      true,
	string(entities.TierStudio):     true,
	string(entities.TierScale):      true,
	string(entities.TierEnterprise): true,
}

// trialTestNamePatterns flag a tenant NAME as likely trial/test debris (prompt
// 046 §3): anonymous-trial*/demo* prefixes, an embedded "smoke", or a long
// trailing unix-timestamp suffix.
var trialTestNamePatterns = []*regexp.Regexp{
	regexp.MustCompile(`(?i)^anonymous-trial`),
	regexp.MustCompile(`(?i)^demo`),
	regexp.MustCompile(`(?i)smoke`),
	regexp.MustCompile(`-\d{10,}$`),
}

// AnalyzeTenantsUseCase computes the read-only analysis report.
type AnalyzeTenantsUseCase struct {
	tenantRepo repositories.TenantRepository
	coreClient clients.CoreClient // may be nil (tests / Core down): Core cross-checks degrade gracefully

	// fleetUC reuses the SAME per-tenant health assessment /fleet/health serves.
	// May be nil: the usage_health health-tier mapping is then skipped (quota
	// pressure still computed from metadata), never failing the run.
	fleetUC *FleetHealthUseCase

	// billingUC reuses the SAME billing-config verifier /billing/config-check
	// serves, for the fleet-level billing finding. May be nil (no LS configured).
	billingUC *VerifyBillingConfigUseCase
}

// NewAnalyzeTenantsUseCase constructs the analysis use case. coreClient,
// fleetUC, and billingUC may each be nil; the relevant cross-checks degrade
// rather than fail.
func NewAnalyzeTenantsUseCase(
	tenantRepo repositories.TenantRepository,
	coreClient clients.CoreClient,
	fleetUC *FleetHealthUseCase,
	billingUC *VerifyBillingConfigUseCase,
) *AnalyzeTenantsUseCase {
	return &AnalyzeTenantsUseCase{
		tenantRepo: tenantRepo,
		coreClient: coreClient,
		fleetUC:    fleetUC,
		billingUC:  billingUC,
	}
}

// AnalyzeRequest selects the scope of an analysis pass.
type AnalyzeRequest struct {
	// Category, when set, runs a single category (data_integrity|plan_billing|
	// litter|usage_health) — powers the per-button calls. Empty = all categories.
	Category string
	// TenantID, when set, analyzes one tenant (fleet findings still computed over
	// the whole fleet for context). Empty = all tenants.
	TenantID string
}

// Execute runs the analysis and returns the report. It NEVER returns a 5xx for a
// single weird tenant: a per-tenant sub-check failure attaches an
// analysis_degraded finding instead. The only hard error is the initial tenant
// list read failing.
func (uc *AnalyzeTenantsUseCase) Execute(ctx context.Context, req AnalyzeRequest) (*dto.AnalysisReport, error) {
	tenants, err := uc.tenantRepo.FindAll()
	if err != nil {
		return nil, err
	}

	wantCat := func(cat string) bool { return req.Category == "" || req.Category == cat }

	// Fleet-wide inputs computed ONCE: the is_demo flag set from Core (litter),
	// and the created_at calendar-date histogram (data_integrity). Both degrade to
	// empty on a Core miss rather than failing the run.
	demoFlags := uc.demoFlagsFromCore(ctx)
	createdAtBuckets := map[string][]string{} // "YYYY-MM-DD" -> tenant ids
	for _, t := range tenants {
		if !t.CreatedAt.IsZero() {
			day := t.CreatedAt.UTC().Format("2006-01-02")
			createdAtBuckets[day] = append(createdAtBuckets[day], t.ID)
		}
	}

	// Duplicate / near-duplicate names (litter): bucket by a normalized name.
	nameBuckets := map[string][]string{}
	for _, t := range tenants {
		nameBuckets[normalizeTenantName(t.Name)] = append(nameBuckets[normalizeTenantName(t.Name)], t.ID)
	}

	now := time.Now()

	// Per-tenant scan (bounded concurrency). Each tenant's analysis is independent
	// and self-contained; a panic or sub-check error degrades that tenant only.
	scope := tenants
	if req.TenantID != "" {
		scope = filterToTenant(tenants, req.TenantID)
	}

	results := make([]dto.AnalysisTenant, len(scope))
	var wg sync.WaitGroup
	sem := make(chan struct{}, analysisMaxConcurrency)
	for i, t := range scope {
		wg.Add(1)
		sem <- struct{}{}
		go func(i int, t *entities.Tenant) {
			defer wg.Done()
			defer func() { <-sem }()
			results[i] = uc.analyzeTenant(ctx, t, tenantAnalysisCtx{
				now:          now,
				isDemo:       demoFlags[t.ID],
				duplicateIDs: nameBuckets[normalizeTenantName(t.Name)],
				wantCat:      wantCat,
			})
		}(i, t)
	}
	wg.Wait()

	// Fleet findings (computed once, attributed to the fleet not a tenant).
	fleet := uc.fleetFindings(ctx, tenants, createdAtBuckets, wantCat)

	return assembleReport(now, results, fleet), nil
}

// tenantAnalysisCtx carries the per-tenant inputs precomputed at the fleet level.
type tenantAnalysisCtx struct {
	now          time.Time
	isDemo       bool
	duplicateIDs []string // tenant ids sharing this tenant's normalized name (incl. self)
	wantCat      func(string) bool
}

// analyzeTenant runs every in-scope category check for one tenant. It recovers
// from a panic in any sub-check and converts it into an analysis_degraded info
// finding so one weird tenant never fails the whole run.
func (uc *AnalyzeTenantsUseCase) analyzeTenant(ctx context.Context, t *entities.Tenant, tctx tenantAnalysisCtx) (out dto.AnalysisTenant) {
	plan := extractPlan(t)
	eventCount := uc.eventCount(ctx, t)
	memberCount := uc.memberCount(ctx, t)

	createdAt := ""
	if !t.CreatedAt.IsZero() {
		createdAt = t.CreatedAt.UTC().Format(time.RFC3339)
	}

	out = dto.AnalysisTenant{
		ID:          t.ID,
		Name:        t.Name,
		Plan:        plan,
		Status:      string(t.Status),
		EventCount:  eventCount,
		MemberCount: memberCount,
		CreatedAt:   createdAt,
		Findings:    []dto.AnalysisFinding{},
	}

	// A sub-check panicking (a malformed metadata shape, a nil deref in a reused
	// helper) degrades THIS tenant only.
	defer func() {
		if r := recover(); r != nil {
			out.Findings = append(out.Findings, dto.AnalysisFinding{
				Category: dto.AnalysisCategoryDataIntegrity,
				Severity: dto.AnalysisSeverityInfo,
				Code:     CodeAnalysisDegraded,
				Title:    "Analysis degraded for this tenant",
				Detail:   fmt.Sprintf("a sub-check failed (%v); other findings for this tenant may be incomplete", r),
			})
			out.WorstSeverity = rollupSeverity(out.Findings)
		}
	}()

	if tctx.wantCat(dto.AnalysisCategoryDataIntegrity) {
		out.Findings = append(out.Findings, uc.dataIntegrityChecks(t, plan, eventCount, tctx.now)...)
	}
	if tctx.wantCat(dto.AnalysisCategoryPlanBilling) {
		out.Findings = append(out.Findings, uc.planBillingChecks(t, plan)...)
	}
	if tctx.wantCat(dto.AnalysisCategoryLitter) {
		out.Findings = append(out.Findings, uc.litterChecks(t, eventCount, memberCount, tctx)...)
	}
	if tctx.wantCat(dto.AnalysisCategoryUsageHealth) {
		out.Findings = append(out.Findings, uc.usageHealthChecks(ctx, t)...)
	}

	out.WorstSeverity = rollupSeverity(out.Findings)
	return out
}

// --- data_integrity ---

func (uc *AnalyzeTenantsUseCase) dataIntegrityChecks(t *entities.Tenant, plan string, eventCount int64, now time.Time) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}

	// Cap artifact: exactly the backfill page cap → a capped value masquerading as
	// a real count. Action: re-run the uncapped reconcile (backfill-usage).
	if eventCount == backfillCountCap {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryDataIntegrity,
			Severity: dto.AnalysisSeverityCritical,
			Code:     CodeCountCapArtifact,
			Title:    "event_count is the backfill cap, not a real count",
			Detail: fmt.Sprintf("event_count == %d exactly — the backfill page cap (2000×500). This is an honest \"≥1M\", "+
				"not a precise count; the 5-min SyncEventsUsageUseCase reconciler (uncapped) supersedes it.", backfillCountCap),
			SuggestedAction: backfillReconcileAction(t.ID),
		})
	}

	// 0 metered events for a tenant past the grace window: genuinely empty OR
	// ingested out-of-band (Prime/MCP/direct Core) and never metered — verify and
	// reconcile. (A capped value is handled above; don't double-flag.)
	if eventCount == 0 && tenantOlderThan(t, now, tenantGraceWindow) {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryDataIntegrity,
			Severity: dto.AnalysisSeverityWarn,
			Code:     CodeUnmeteredOrEmpty,
			Title:    "0 metered events",
			Detail: "either genuinely empty or ingested out-of-band (Prime/MCP/direct Core) and never metered. " +
				"Verify against Core's real event log; if real, reconcile the meter via backfill-usage.",
			SuggestedAction: backfillReconcileAction(t.ID),
		})
	}

	// Missing/zero required fields: no plan, or a zero-time created_at.
	missing := []string{}
	if strings.TrimSpace(plan) == "" {
		missing = append(missing, "plan")
	}
	if t.CreatedAt.IsZero() {
		missing = append(missing, "created_at")
	}
	if len(missing) > 0 {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryDataIntegrity,
			Severity: dto.AnalysisSeverityWarn,
			Code:     CodeMissingFields,
			Title:    "missing/zero required fields",
			Detail:   "tenant is missing or has zero-value required fields: " + strings.Join(missing, ", "),
		})
	}

	// Plan shape drift: the `plan`/`subscription` metadata is present but not a
	// clean string/known shape. The check code must NOT assume `plan` is a string
	// (the DETAIL DTO emits {name,tier}, the LIST DTO a bare string — that drift
	// crashed the admin 360). Surface it as a data_integrity finding.
	if drift, ok := planShapeDrift(t); ok {
		out = append(out, dto.AnalysisFinding{
			Category:        dto.AnalysisCategoryDataIntegrity,
			Severity:        dto.AnalysisSeverityWarn,
			Code:            CodePlanShapeDrift,
			Title:           "plan representation is inconsistent",
			Detail:          drift,
			SuggestedAction: linkAction("Verify billing config", "/api/v1/admin/billing/config-check"),
		})
	}

	return out
}

// --- plan_billing ---

func (uc *AnalyzeTenantsUseCase) planBillingChecks(t *entities.Tenant, plan string) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}

	// Tier not in the canonical catalog (after retired-alias normalization).
	normalized := entities.MapRetiredTier(plan)
	if strings.TrimSpace(plan) != "" && !canonicalTiers[normalized] {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryPlanBilling,
			Severity: dto.AnalysisSeverityCritical,
			Code:     CodePlanNotInCatalog,
			Title:    "plan not in catalog",
			Detail: fmt.Sprintf("tenant plan %q is not a canonical tier (free/indie/studio/scale/enterprise) "+
				"nor a known retired alias; checkout/entitlements cannot resolve it.", plan),
			SuggestedAction: linkAction("Verify billing config", "/api/v1/admin/billing/config-check"),
		})
	}

	// Paid plan but no active subscription metadata. Reuses the SAME subscription
	// extraction the fleet-health model uses (extractSubscriptionForHealth) — a
	// paid effective tier with no active subscription status is the
	// "paid_plan_no_subscription" smell. (Enterprise is sales-led / comp, so its
	// subscription may legitimately be absent — exclude it from the alarm.)
	sub := extractSubscriptionForHealth(t.Metadata)
	effTier := effectiveBillingTier(t.Metadata, sub)
	if isAlarmablePaidTier(effTier) && !entities.SubscriptionIsActive(sub.Status) {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryPlanBilling,
			Severity: dto.AnalysisSeverityWarn,
			Code:     CodePaidPlanNoSubscription,
			Title:    "paid plan without an active subscription",
			Detail: fmt.Sprintf("effective tier %q is paid but the subscription status is %q (not active/trialing/past_due) "+
				"and no matching LemonSqueezy subscription is recorded; entitlements may be granted without billing.",
				effTier, sub.Status),
			SuggestedAction: linkAction("Reconcile subscription", "/api/v1/admin/recovery/"+t.ID+"/reconcile-subscription"),
		})
	}

	return out
}

// --- litter ---

func (uc *AnalyzeTenantsUseCase) litterChecks(t *entities.Tenant, eventCount int64, memberCount int, tctx tenantAnalysisCtx) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}

	// is_demo (from Core's flag) → reap-demo.
	if tctx.isDemo {
		out = append(out, dto.AnalysisFinding{
			Category:        dto.AnalysisCategoryLitter,
			Severity:        dto.AnalysisSeverityWarn,
			Code:            CodeDemoLitter,
			Title:           "demo tenant (is_demo)",
			Detail:          "Core flagged this tenant is_demo; it is demo litter from the status-probe side effect. Reap it (dry-run first).",
			SuggestedAction: linkAction("Reap demo tenants (dry-run first)", "/api/v1/admin/tenants/reap-demo"),
		})
	}

	// Name matches a trial/test pattern.
	if matchesTrialTestName(t.Name) {
		out = append(out, dto.AnalysisFinding{
			Category: dto.AnalysisCategoryLitter,
			Severity: dto.AnalysisSeverityInfo,
			Code:     CodeTrialOrTestTenant,
			Title:    "name looks like a trial/test tenant",
			Detail:   fmt.Sprintf("tenant name %q matches a trial/test pattern (anonymous-trial*/demo*/smoke/timestamp-suffix).", t.Name),
		})
	}

	// Duplicate / near-duplicate name (more than one tenant shares the normalized name).
	if len(tctx.duplicateIDs) > 1 {
		out = append(out, dto.AnalysisFinding{
			Category:      dto.AnalysisCategoryLitter,
			Severity:      dto.AnalysisSeverityInfo,
			Code:          CodeDuplicateName,
			Title:         "duplicate / near-duplicate name",
			Detail:        fmt.Sprintf("%d tenants share the normalized name %q: %s", len(tctx.duplicateIDs), normalizeTenantName(t.Name), strings.Join(tctx.duplicateIDs, ", ")),
			AffectedCount: len(tctx.duplicateIDs),
		})
	}

	// Empty debris: 0 events AND 0 members, past the grace window.
	if eventCount == 0 && memberCount == 0 && tenantOlderThan(t, tctx.now, tenantGraceWindow) {
		out = append(out, dto.AnalysisFinding{
			Category:        dto.AnalysisCategoryLitter,
			Severity:        dto.AnalysisSeverityWarn,
			Code:            CodeEmptyTenant,
			Title:           "empty tenant (likely debris)",
			Detail:          "0 events and 0 members, older than the grace window — likely debris. Reap (dry-run first) after confirming.",
			SuggestedAction: linkAction("Reap demo tenants (dry-run first)", "/api/v1/admin/tenants/reap-demo"),
		})
	}

	return out
}

// --- usage_health ---

func (uc *AnalyzeTenantsUseCase) usageHealthChecks(ctx context.Context, t *entities.Tenant) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}

	// Quota pressure from the metadata mirror (the same events_used/events_quota
	// the fleet-health quota signal reads). >= 0.9 warn, >= 1.0 critical.
	q := extractQuotaForHealth(t.Metadata)
	if q.EventsQuota > 0 {
		ratio := float64(q.EventsUsed) / float64(q.EventsQuota)
		switch {
		case ratio >= quotaPressureCritical:
			out = append(out, dto.AnalysisFinding{
				Category:        dto.AnalysisCategoryUsageHealth,
				Severity:        dto.AnalysisSeverityCritical,
				Code:            CodeQuotaPressure,
				Title:           "over events quota",
				Detail:          fmt.Sprintf("events_used/events_quota = %d/%d (%.0f%%) — at or over quota.", q.EventsUsed, q.EventsQuota, ratio*100),
				SuggestedAction: linkAction("Reconcile subscription / review quota", "/api/v1/admin/recovery/"+t.ID+"/reconcile-subscription"),
			})
		case ratio >= quotaPressureWarn:
			out = append(out, dto.AnalysisFinding{
				Category:        dto.AnalysisCategoryUsageHealth,
				Severity:        dto.AnalysisSeverityWarn,
				Code:            CodeQuotaPressure,
				Title:           "approaching events quota",
				Detail:          fmt.Sprintf("events_used/events_quota = %d/%d (%.0f%%) — approaching quota.", q.EventsUsed, q.EventsQuota, ratio*100),
				SuggestedAction: linkAction("Reconcile subscription / review quota", "/api/v1/admin/recovery/"+t.ID+"/reconcile-subscription"),
			})
		}
	}

	// Reuse the SAME per-tenant assessment /fleet/health serves: an at_risk /
	// critical health tier becomes a usage_health finding deep-linking to the
	// per-tenant health view. fleetUC may be nil (skipped, not failed).
	if uc.fleetUC != nil {
		if res, err := uc.fleetUC.ComputeTenant(ctx, t.ID); err == nil && res != nil {
			sev := healthTierToSeverity(res.Tier)
			if sev != "" {
				out = append(out, dto.AnalysisFinding{
					Category:        dto.AnalysisCategoryUsageHealth,
					Severity:        sev,
					Code:            CodeHealthAtRisk,
					Title:           "fleet-health tier " + res.Tier,
					Detail:          fleetHealthReasonDetail(res),
					SuggestedAction: linkAction("Open tenant health", "/api/v1/admin/fleet/health/"+t.ID),
				})
			}
			if stale, detail := staleFromHealth(res); stale {
				out = append(out, dto.AnalysisFinding{
					Category:        dto.AnalysisCategoryUsageHealth,
					Severity:        dto.AnalysisSeverityInfo,
					Code:            CodeStaleTenant,
					Title:           "no recent events",
					Detail:          detail,
					SuggestedAction: linkAction("Open tenant health", "/api/v1/admin/fleet/health/"+t.ID),
				})
			}
		}
	}

	return out
}

// --- fleet-wide findings ---

func (uc *AnalyzeTenantsUseCase) fleetFindings(ctx context.Context, tenants []*entities.Tenant, createdAtBuckets map[string][]string, wantCat func(string) bool) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}

	// data_integrity: created_at-not-real. >= threshold tenants sharing ONE date
	// is the time.Now()-on-load smell (fixed 2026-06-26; this stays a regression
	// guard).
	if wantCat(dto.AnalysisCategoryDataIntegrity) {
		worstDay, worstIDs := "", []string{}
		for day, ids := range createdAtBuckets {
			if len(ids) > len(worstIDs) {
				worstDay, worstIDs = day, ids
			}
		}
		// Fire only when the dominant date bucket covers most of the fleet (the
		// bug stamps the SAME now() on every tenant), or there is effectively one
		// distinct date. A same-day batch that is a minority of many real dates is
		// legitimate, not the bug — see createdAtDominanceFraction.
		dominates := len(tenants) > 0 &&
			float64(len(worstIDs))/float64(len(tenants)) >= createdAtDominanceFraction
		oneDistinctDate := len(createdAtBuckets) <= 1
		if len(worstIDs) >= createdAtSharedDateThreshold && (dominates || oneDistinctDate) {
			out = append(out, dto.AnalysisFinding{
				Category:      dto.AnalysisCategoryDataIntegrity,
				Severity:      dto.AnalysisSeverityCritical,
				Code:          CodeCreatedAtNotReal,
				Title:         "created_at is not real (time.Now()-on-load)",
				Detail:        fmt.Sprintf("%d of %d tenants share the created_at date %s — the stamped-in-one-loop smell. The column is not backed by each tenant's real creation time.", len(worstIDs), len(tenants), worstDay),
				AffectedCount: len(worstIDs),
			})
		}
	}

	// plan_billing: distribution suspicious + billing config broken.
	if wantCat(dto.AnalysisCategoryPlanBilling) {
		out = append(out, uc.planDistributionFinding(tenants)...)
		out = append(out, uc.billingConfigFinding()...)
	}

	return out
}

// planDistributionFinding flags a fleet whose plans all collapse to one tier, or
// where free tenants carry paid entitlements (the "plans mapped unexpectedly"
// complaint at the fleet level).
func (uc *AnalyzeTenantsUseCase) planDistributionFinding(tenants []*entities.Tenant) []dto.AnalysisFinding {
	out := []dto.AnalysisFinding{}
	if len(tenants) < createdAtSharedDateThreshold {
		return out // too few tenants to call a distribution "suspicious"
	}

	dist := map[string]int{}
	freeWithEntitlements := 0
	for _, t := range tenants {
		plan := entities.MapRetiredTier(extractPlan(t))
		dist[plan]++
		// A free tenant whose effective billing tier is paid carries paid
		// entitlements without a free plan — the inverse mapping smell.
		if plan == string(entities.TierFree) {
			sub := extractSubscriptionForHealth(t.Metadata)
			if isAlarmablePaidTier(effectiveBillingTier(t.Metadata, sub)) {
				freeWithEntitlements++
			}
		}
	}

	// Every tenant on ONE plan (and that plan is not free — an all-free fleet is
	// normal pre-launch) is suspicious.
	if len(dist) == 1 {
		for plan := range dist {
			if plan != string(entities.TierFree) {
				out = append(out, dto.AnalysisFinding{
					Category:        dto.AnalysisCategoryPlanBilling,
					Severity:        dto.AnalysisSeverityWarn,
					Code:            CodePlanDistributionSuspect,
					Title:           "every tenant collapses to one plan",
					Detail:          fmt.Sprintf("all %d tenants map to plan %q — a single-plan fleet is almost always a mapping bug, not real billing.", len(tenants), plan),
					AffectedCount:   len(tenants),
					SuggestedAction: linkAction("Verify billing config", "/api/v1/admin/billing/config-check"),
				})
			}
		}
	}

	if freeWithEntitlements > 0 {
		out = append(out, dto.AnalysisFinding{
			Category:        dto.AnalysisCategoryPlanBilling,
			Severity:        dto.AnalysisSeverityWarn,
			Code:            CodePlanDistributionSuspect,
			Title:           "free tenants carry paid entitlements",
			Detail:          fmt.Sprintf("%d tenant(s) on the free plan have a paid effective billing tier — entitlements granted without a paid plan.", freeWithEntitlements),
			AffectedCount:   freeWithEntitlements,
			SuggestedAction: linkAction("Verify billing config", "/api/v1/admin/billing/config-check"),
		})
	}

	return out
}

// billingConfigFinding reuses the SAME billing-config verifier /billing/config-check
// serves; an error-severity report becomes a fleet finding. billingUC may be nil
// (no LS configured) → no finding.
func (uc *AnalyzeTenantsUseCase) billingConfigFinding() []dto.AnalysisFinding {
	if uc.billingUC == nil {
		return nil
	}
	report := uc.billingUC.Execute()
	if report.Skipped || report.OK {
		return nil
	}
	errCount := 0
	for _, iss := range report.Issues {
		if iss.Severity == SeverityError {
			errCount++
		}
	}
	if errCount == 0 {
		return nil
	}
	return []dto.AnalysisFinding{{
		Category:        dto.AnalysisCategoryPlanBilling,
		Severity:        dto.AnalysisSeverityCritical,
		Code:            CodeBillingConfigBroken,
		Title:           "billing config has errors",
		Detail:          fmt.Sprintf("%d error-severity billing-config issue(s) detected; paid checkouts may silently fall back to free. See config-check for details.", errCount),
		AffectedCount:   errCount,
		SuggestedAction: linkAction("Open billing config check", "/api/v1/admin/billing/config-check"),
	}}
}

// --- counts (reuse list_tenants sourcing) ---

// eventCount sources the tenant's event total: live Core stats cross-check when a
// client is wired (capped work, single cheap call), falling back to the metadata
// mirror (extractEventCount) on any miss. Mirrors list_tenants.eventCountForTenant.
func (uc *AnalyzeTenantsUseCase) eventCount(ctx context.Context, t *entities.Tenant) int64 {
	if uc.coreClient != nil {
		if stats, err := uc.coreClient.GetTenantStats(ctx, t.ID); err == nil && stats != nil {
			return stats.EventCount
		}
	}
	return extractEventCount(t)
}

// memberCount mirrors list_tenants.memberCountForTenant: real team-members list
// from Core, falling back to the metadata mirror.
func (uc *AnalyzeTenantsUseCase) memberCount(ctx context.Context, t *entities.Tenant) int {
	if uc.coreClient != nil {
		if n, ok := memberCountFromCore(ctx, uc.coreClient, t.ID); ok {
			return n
		}
	}
	return extractMemberCount(t)
}

// demoFlagsFromCore reads Core's tenant list once and returns the set of
// is_demo tenant ids. Degrades to an empty map (no demo findings) when no Core
// client is wired or Core errors — never fails the run.
func (uc *AnalyzeTenantsUseCase) demoFlagsFromCore(ctx context.Context) map[string]bool {
	flags := map[string]bool{}
	if uc.coreClient == nil {
		return flags
	}
	resp, err := uc.coreClient.ListTenants(ctx)
	if err != nil || resp == nil {
		return flags
	}
	for _, t := range resp.Tenants {
		if t.IsDemo {
			flags[t.ID] = true
		}
	}
	return flags
}

// --- report assembly + small helpers ---

// assembleReport rolls the per-tenant + fleet findings into the summary and
// sorts tenants worst-first for stable rendering. Tenants with no findings are
// retained (worst_severity="ok") so the UI can show the full fleet.
func assembleReport(now time.Time, tenants []dto.AnalysisTenant, fleet []dto.AnalysisFinding) *dto.AnalysisReport {
	byCat := map[string]int{
		dto.AnalysisCategoryDataIntegrity: 0,
		dto.AnalysisCategoryPlanBilling:   0,
		dto.AnalysisCategoryLitter:        0,
		dto.AnalysisCategoryUsageHealth:   0,
	}
	bySev := map[string]int{
		dto.AnalysisSeverityCritical: 0,
		dto.AnalysisSeverityWarn:     0,
		dto.AnalysisSeverityInfo:     0,
	}
	flagged := 0
	count := func(f dto.AnalysisFinding) {
		byCat[f.Category]++
		bySev[f.Severity]++
	}
	for _, t := range tenants {
		if len(t.Findings) > 0 {
			flagged++
		}
		for _, f := range t.Findings {
			count(f)
		}
	}
	for _, f := range fleet {
		count(f)
	}

	// Worst-first ordering for the tenants slice (stable by id within a severity).
	sort.SliceStable(tenants, func(i, j int) bool {
		si, sj := severityRank(tenants[i].WorstSeverity), severityRank(tenants[j].WorstSeverity)
		if si != sj {
			return si > sj
		}
		return tenants[i].ID < tenants[j].ID
	})

	if fleet == nil {
		fleet = []dto.AnalysisFinding{}
	}

	return &dto.AnalysisReport{
		GeneratedAt: now.UTC().Format(time.RFC3339),
		Summary: dto.AnalysisSummary{
			TotalTenants:   len(tenants),
			FlaggedTenants: flagged,
			ByCategory:     byCat,
			BySeverity:     bySev,
		},
		FleetFindings: fleet,
		Tenants:       tenants,
	}
}

// rollupSeverity returns the worst severity among a tenant's findings, or "ok".
func rollupSeverity(findings []dto.AnalysisFinding) string {
	worst := dto.AnalysisSeverityOK
	for _, f := range findings {
		if severityRank(f.Severity) > severityRank(worst) {
			worst = f.Severity
		}
	}
	return worst
}

// severityRank orders severities worst→best (higher = worse).
func severityRank(sev string) int {
	switch sev {
	case dto.AnalysisSeverityCritical:
		return 3
	case dto.AnalysisSeverityWarn:
		return 2
	case dto.AnalysisSeverityInfo:
		return 1
	default: // ok / unknown
		return 0
	}
}

// healthTierToSeverity maps a FleetHealth tier to an analysis severity. Only
// at_risk/critical surface as findings; degraded/healthy do not (they're not
// data anomalies for this report).
func healthTierToSeverity(tier string) string {
	switch tier {
	case "critical":
		return dto.AnalysisSeverityCritical
	case "at_risk":
		return dto.AnalysisSeverityWarn
	default:
		return ""
	}
}

// fleetHealthReasonDetail renders the non-healthy signal reasons from a tenant
// health result into a one-line detail.
func fleetHealthReasonDetail(res *TenantHealthResult) string {
	reasons := []string{}
	for _, s := range res.Signals {
		if s.Tier != "" && string(s.Tier) != "healthy" {
			reasons = append(reasons, fmt.Sprintf("%s=%s", s.Name, s.Value))
		}
	}
	if len(reasons) == 0 {
		return "fleet-health tier " + res.Tier
	}
	return "fleet-health tier " + res.Tier + ": " + strings.Join(reasons, "; ")
}

// staleFromHealth reads the last_event_age signal from a tenant health result and
// reports staleness (a degraded/at_risk recency) as an info finding.
func staleFromHealth(res *TenantHealthResult) (bool, string) {
	for _, s := range res.Signals {
		if s.Name == "last_event_age" && s.Tier != "" && string(s.Tier) != "healthy" && s.Value != "never ingested" {
			return true, "no recent events (last event " + s.Value + " ago)"
		}
	}
	return false, ""
}

// isAlarmablePaidTier reports whether a tier is paid AND self-serve (so a missing
// subscription is a real anomaly). Enterprise is sales-led/comp — excluded.
func isAlarmablePaidTier(tier string) bool {
	switch entities.SubscriptionTier(entities.MapRetiredTier(tier)) {
	case entities.TierIndie, entities.TierStudio, entities.TierScale:
		return true
	default:
		return false
	}
}

// planShapeDrift inspects the raw `subscription`/`plan` metadata WITHOUT assuming
// `plan` is a string. It returns a human description when the shape is suspect
// (e.g. a non-string flat plan key, or a subscription that is neither a known
// struct nor a map), else ("", false).
func planShapeDrift(t *entities.Tenant) (string, bool) {
	if t.Metadata == nil {
		return "", false
	}
	// A flat top-level "plan" key that isn't a string is shape drift (the DTO
	// emits a bare tier string at the list level — anything else would crash a
	// consumer expecting a string).
	if raw, ok := t.Metadata["plan"]; ok {
		if _, isStr := raw.(string); !isStr {
			return fmt.Sprintf("metadata.plan is %T, expected a string tier (DTO shape drift)", raw), true
		}
	}
	// The nested subscription, when present, must be a known struct or a JSON map.
	if raw, ok := t.Metadata["subscription"]; ok && raw != nil {
		switch raw.(type) {
		case *entities.SubscriptionMetadata, entities.SubscriptionMetadata, map[string]interface{}:
			// fine
		default:
			return fmt.Sprintf("metadata.subscription is %T, not a known subscription shape", raw), true
		}
	}
	return "", false
}

// tenantOlderThan reports whether the tenant was created more than d ago. A
// zero/unknown created_at is treated as "old enough to judge" so a tenant with a
// missing timestamp isn't excused from the 0-count/empty checks.
func tenantOlderThan(t *entities.Tenant, now time.Time, d time.Duration) bool {
	if t.CreatedAt.IsZero() {
		return true
	}
	return now.Sub(t.CreatedAt) > d
}

// matchesTrialTestName reports whether a tenant name matches any trial/test pattern.
func matchesTrialTestName(name string) bool {
	for _, re := range trialTestNamePatterns {
		if re.MatchString(name) {
			return true
		}
	}
	return false
}

// normalizeTenantName lowercases + trims a name for duplicate detection.
func normalizeTenantName(name string) string {
	return strings.ToLower(strings.TrimSpace(name))
}

// filterToTenant returns the single-tenant slice (or empty if not found).
func filterToTenant(tenants []*entities.Tenant, id string) []*entities.Tenant {
	for _, t := range tenants {
		if t.ID == id {
			return []*entities.Tenant{t}
		}
	}
	return []*entities.Tenant{}
}

// --- suggested-action builders (all targets are EXISTING routes/commands) ---

func linkAction(label, target string) *dto.SuggestedAction {
	return &dto.SuggestedAction{Label: label, Kind: dto.AnalysisActionKindLink, Target: target}
}

// backfillReconcileAction deep-links the count-reconcile remediation. The HTTP
// route (POST /api/v1/admin/billing/backfill-usage) is link-kind; the operator
// Taskfile equivalent is noted in the label for CLI-driven reconciles.
func backfillReconcileAction(tenantID string) *dto.SuggestedAction {
	return &dto.SuggestedAction{
		Label:  "Reconcile events_used from Core (task backfill-usage TENANT=" + tenantID + " DRY=false)",
		Kind:   dto.AnalysisActionKindLink,
		Target: "/api/v1/admin/billing/backfill-usage",
	}
}
