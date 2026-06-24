package usecases

import (
	"context"
)

// ReapDemoMaxTenants is the hard ceiling on how many demo tenants a single reap
// may delete. The demo-litter probe created roughly one tenant per status poll,
// so the realistic backlog is dozens — but a cohort delete still goes through a
// blast-radius cap (mirroring BatchAbsoluteCeiling) so a buggy is_demo flag on
// the whole fleet can never wipe every tenant in one call. A backlog larger than
// this is reaped in successive runs, each independently dry-run + confirmed.
const ReapDemoMaxTenants = BatchAbsoluteCeiling

// ReapDemoCandidate is one demo tenant surfaced by the reap dry-run preview.
type ReapDemoCandidate struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// ReapDemoResult is the response for POST /api/v1/admin/tenants/reap-demo. On
// dry_run it lists the matched demo tenants + count + a confirm_token; on apply
// it reports how many were deleted.
type ReapDemoResult struct {
	Action       string              `json:"action"`
	DryRun       bool                `json:"dry_run"`
	Applied      bool                `json:"applied"`
	MatchedTotal int                 `json:"matched_total"`
	Deleted      int                 `json:"deleted"`
	Candidates   []ReapDemoCandidate `json:"candidates"`
	ConfirmToken string              `json:"confirm_token,omitempty"`
	Message      string              `json:"message,omitempty"`
}

// reapDemoTokenScope binds the confirm_token to the matched count so a token
// minted for one preview can't be replayed once the demo backlog has grown
// (same defence as batchTokenScope).
func reapDemoTokenScope(count int) string {
	return "demo:" + itoaInt(count)
}

// ReapDemo finds the tenants Core flagged is_demo and deletes them, applying the
// EXACT recovery guard discipline used by Batch: dry-run returns the matched
// list + count + a confirm_token, and apply requires that echoed token. This is
// the cleanup half of gap #1 (the demo-litter the side-effectful status probe
// minted) — the prevention half is the DEMO_ENABLED gate on DemoStartHandler.
//
// Source of the is_demo flag: CORE, not the CP's local tenant mirror. The flag
// lives on Core's tenant record (TenantResponse.IsDemo) and is surfaced by
// coreClient.ListTenants(); entities.Tenant carries no IsDemo field. Deletion is
// a Core tenant delete (DELETE /api/v1/tenants/{id}) — no new DB, audited as a
// Core event. Reaping is bounded by ReapDemoMaxTenants.
func (uc *RecoveryUseCase) ReapDemo(ctx context.Context, req RecoveryRequest) (*ReapDemoResult, error) {
	const action = "reap_demo"

	candidates, err := uc.listDemoTenants(ctx)
	if err != nil {
		return nil, err
	}

	// Blast-radius cap: a fleet-wide is_demo glitch must not wipe everything in
	// one call. The operator narrows the field (reaps in successive runs) rather
	// than silently deleting the first N of a huge matched set.
	if len(candidates) > ReapDemoMaxTenants {
		return nil, ErrBatchTooLarge
	}

	result := &ReapDemoResult{
		Action:       action,
		DryRun:       req.DryRun,
		MatchedTotal: len(candidates),
		Candidates:   candidates,
	}

	if req.DryRun {
		// Token-gated preview: echo this token on apply. Scoped to the matched
		// count so it can't be replayed against a different backlog size.
		result.ConfirmToken = uc.guard.mintConfirmToken(action, reapDemoTokenScope(len(candidates)))
		_ = uc.auditor.Record(ctx, RecoveryAuditRecord{ //nolint:errcheck // preview audit is best-effort; no state changed
			Action: action, TenantID: RecoveryAuditTenant, Actor: req.Actor, DryRun: true,
			Reason: req.Reason, Outcome: "preview",
			Details: map[string]any{"matched_total": len(candidates), "ids": candidateIDs(candidates)},
		})
		result.Message = "dry-run: echo confirm_token to apply"
		return result, nil
	}

	// Apply guard: echoed confirm_token for THIS matched count.
	if req.ConfirmToken == "" {
		return nil, ErrConfirmRequired
	}
	if !uc.guard.validateConfirmToken(action, reapDemoTokenScope(len(candidates)), req.ConfirmToken) {
		return nil, ErrConfirmMismatch
	}

	// Audit BEFORE the deletes so the intent (count + ids) is durable even if a
	// delete partially completes — exactly one admin.recovery.reap_demo event.
	if err := uc.auditor.Record(ctx, RecoveryAuditRecord{
		Action: action, TenantID: RecoveryAuditTenant, Actor: req.Actor, DryRun: false, Reason: req.Reason,
		Outcome: "applied",
		Details: map[string]any{"matched_total": len(candidates), "ids": candidateIDs(candidates)},
	}); err != nil {
		return nil, err
	}

	// Delete each matched demo tenant in Core. A per-tenant delete failure is
	// surfaced in Deleted (< MatchedTotal) rather than aborting the whole reap.
	if uc.coreClient != nil {
		for _, cnd := range candidates {
			if delErr := uc.coreClient.DeleteTenant(ctx, cnd.ID); delErr == nil {
				result.Deleted++
			}
		}
	}

	result.Applied = true
	result.Message = "demo tenants reaped"
	return result, nil
}

// listDemoTenants lists tenants from Core and keeps only those Core flagged
// is_demo. Returns an empty slice (never nil candidates) when no Core client is
// wired (test mode) or nothing matches.
func (uc *RecoveryUseCase) listDemoTenants(ctx context.Context) ([]ReapDemoCandidate, error) {
	out := []ReapDemoCandidate{}
	if uc.coreClient == nil {
		return out, nil
	}
	resp, err := uc.coreClient.ListTenants(ctx)
	if err != nil {
		return nil, err
	}
	if resp == nil {
		return out, nil
	}
	for _, t := range resp.Tenants {
		if t.IsDemo {
			out = append(out, ReapDemoCandidate{ID: t.ID, Name: t.Name})
		}
	}
	return out, nil
}

func candidateIDs(cs []ReapDemoCandidate) []string {
	ids := make([]string, 0, len(cs))
	for _, c := range cs {
		ids = append(ids, c.ID)
	}
	return ids
}
