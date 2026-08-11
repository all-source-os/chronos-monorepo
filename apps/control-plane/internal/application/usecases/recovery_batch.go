package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// BatchFilter selects the tenants a batch recovery applies to. All set fields
// must match (AND). HealthTier filtering is intentionally NOT supported here
// (it would require a full fleet assessment per batch); status/tier are the
// cheap selectors that cover the runbook's "retired-tier backfill" use case.
type BatchFilter struct {
	Status string `json:"status,omitempty"` // tenant status (active/suspended/…)
	Tier   string `json:"tier,omitempty"`   // billing tier (e.g. "pro" for retired-tier backfill)
}

// BatchRequest is the body for POST /api/v1/admin/recovery/batch.
type BatchRequest struct {
	RecoveryRequest
	Filter     BatchFilter `json:"filter"`
	Action     string      `json:"action"`      // Guarded sub-action only
	MaxTenants int         `json:"max_tenants"` // hard cap; default 25, ceiling 100
}

// BatchTenantOutcome is the per-tenant result inside a batch.
type BatchTenantOutcome struct {
	TenantID string `json:"tenant_id"`
	Outcome  string `json:"outcome"` // "preview" | "applied" | "error"
	Error    string `json:"error,omitempty"`
}

// BatchResult is the response for a batch recovery.
type BatchResult struct {
	Action       string               `json:"action"`
	DryRun       bool                 `json:"dry_run"`
	Applied      bool                 `json:"applied"`
	MatchedTotal int                  `json:"matched_total"` // total filter matches (pre-cap)
	Affected     int                  `json:"affected"`      // count within the cap
	MaxTenants   int                  `json:"max_tenants"`
	ConfirmToken string               `json:"confirm_token,omitempty"`
	Tenants      []BatchTenantOutcome `json:"tenants"`
	Message      string               `json:"message,omitempty"`
}

// batchAllowedActions are the Guarded sub-actions a batch may run. Destructive
// actions (rotate_keys, reprovision, restore) are FORBIDDEN in batch (§4.3).
var batchAllowedActions = map[string]bool{
	"resync":                 true,
	"force_resync":           true,
	"reconcile_subscription": true,
	"resolve_dunning":        true,
}

// Batch applies one Guarded sub-action across the filtered tenants. Guards:
//   - Destructive sub-actions are hard-rejected (ErrBatchDestructiveForbidden).
//   - The affected count is capped at min(max_tenants, ceiling 100); a filter
//     matching > ceiling is rejected (ErrBatchTooLarge) so an unbounded "recover
//     everything" can never run.
//   - dry_run returns the full affected list + a confirm_token echoing the count.
//   - apply requires the echoed confirm_token (token-gated).
//
// One audit event is written for the batch itself; each per-tenant sub-action
// writes its own audit via the underlying Guarded use case.
func (uc *RecoveryUseCase) Batch(ctx context.Context, req BatchRequest) (*BatchResult, error) {
	const action = "batch"

	// Normalize the sub-action and reject Destructive ones up front.
	sub := normalizeBatchAction(req.Action)
	if !batchAllowedActions[sub] {
		return nil, ErrBatchDestructiveForbidden
	}

	// Resolve the cap: default 25, absolute ceiling 100.
	maxT := req.MaxTenants
	if maxT <= 0 {
		maxT = BatchDefaultMaxTenants
	}
	if maxT > BatchAbsoluteCeiling {
		return nil, ErrBatchTooLarge
	}

	// Select tenants by filter.
	matched, err := uc.matchTenants(req.Filter)
	if err != nil {
		return nil, err
	}
	// A filter that matches more than the ceiling is rejected — the operator must
	// narrow the filter, not silently mutate the first 100 of 5000.
	if len(matched) > BatchAbsoluteCeiling {
		return nil, ErrBatchTooLarge
	}

	affected := matched
	if len(affected) > maxT {
		affected = affected[:maxT]
	}

	result := &BatchResult{
		Action:       action,
		DryRun:       req.DryRun,
		MatchedTotal: len(matched),
		Affected:     len(affected),
		MaxTenants:   maxT,
		Tenants:      make([]BatchTenantOutcome, 0, len(affected)),
	}

	if req.DryRun {
		for _, t := range affected {
			result.Tenants = append(result.Tenants, BatchTenantOutcome{TenantID: t.ID, Outcome: "preview"})
		}
		// Token bound to the batch action + a synthetic id encoding the count, so
		// the apply must echo a token minted for THIS affected count.
		result.ConfirmToken = uc.guard.mintConfirmToken(action, batchTokenScope(sub, len(affected)))
		_ = uc.auditor.Record(ctx, RecoveryAuditRecord{ //nolint:errcheck // preview audit best-effort
			Action: action, TenantID: RecoveryAuditTenant, Actor: req.Actor, DryRun: true,
			Reason: req.Reason, Outcome: "preview",
			Details: map[string]any{"sub_action": sub, "affected": len(affected), "matched_total": len(matched)},
		})
		result.Message = dryRunPreviewMessage
		return result, nil
	}

	// Apply guard: echoed confirm_token for THIS (action, sub, count) scope.
	if req.ConfirmToken == "" {
		return nil, ErrConfirmRequired
	}
	if !uc.guard.validateConfirmToken(action, batchTokenScope(sub, len(affected)), req.ConfirmToken) {
		return nil, ErrConfirmMismatch
	}

	// One batch-level audit event before fan-out (§4.3 "one admin.recovery.batch
	// event + one per affected tenant").
	if err := uc.auditor.Record(ctx, RecoveryAuditRecord{
		Action: action, TenantID: RecoveryAuditTenant, Actor: req.Actor, DryRun: false, Reason: req.Reason,
		Outcome: "applied",
		Details: map[string]any{"sub_action": sub, "affected": len(affected), "matched_total": len(matched)},
	}); err != nil {
		return nil, err
	}

	// Fan out the Guarded sub-action per tenant (each writes its own audit).
	for _, t := range affected {
		subReq := RecoveryRequest{DryRun: false, Reason: req.Reason, Actor: req.Actor}
		var subErr error
		switch sub {
		case "resync", "force_resync":
			_, subErr = uc.ForceResync(ctx, t.ID, subReq)
		case "reconcile_subscription":
			_, subErr = uc.ReconcileSubscription(ctx, t.ID, subReq)
		case "resolve_dunning":
			_, subErr = uc.ResolveDunning(ctx, t.ID, subReq)
		}
		out := BatchTenantOutcome{TenantID: t.ID, Outcome: "applied"}
		if subErr != nil {
			out.Outcome = "error"
			out.Error = subErr.Error()
		}
		result.Tenants = append(result.Tenants, out)
	}

	result.Applied = true
	result.Message = "batch applied"
	return result, nil
}

// matchTenants returns the tenants matching the filter.
func (uc *RecoveryUseCase) matchTenants(f BatchFilter) ([]*entities.Tenant, error) {
	all, err := uc.tenantRepo.FindAll()
	if err != nil {
		return nil, err
	}
	out := make([]*entities.Tenant, 0, len(all))
	wantStatus := normalizeDunningStatus(f.Status)
	wantTier := normalizeDunningStatus(f.Tier)
	for _, t := range all {
		if wantStatus != "" && normalizeDunningStatus(string(t.Status)) != wantStatus {
			continue
		}
		if wantTier != "" {
			sub := extractSubscriptionForHealth(t.Metadata)
			tier := effectiveBillingTier(t.Metadata, sub)
			if normalizeDunningStatus(tier) != wantTier {
				continue
			}
		}
		out = append(out, t)
	}
	return out, nil
}

func normalizeBatchAction(a string) string {
	return normalizeDunningStatus(a)
}

// batchTokenScope encodes the sub-action + affected count into the token scope so
// a token minted for one count can't be replayed against a different count.
func batchTokenScope(sub string, count int) string {
	return sub + ":" + itoaInt(count)
}

func itoaInt(n int) string {
	if n == 0 {
		return "0"
	}
	var b [20]byte
	i := len(b)
	neg := n < 0
	if neg {
		n = -n
	}
	for n > 0 {
		i--
		b[i] = byte('0' + n%10)
		n /= 10
	}
	if neg {
		i--
		b[i] = '-'
	}
	return string(b[i:])
}
