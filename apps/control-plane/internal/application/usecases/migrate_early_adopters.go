package usecases

import (
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// defaultVoucherTier / defaultVoucherDays are applied when a request omits them.
const (
	defaultVoucherTier = "studio"
	defaultVoucherDays = 365
)

// EarlyAdopterResult records what happened to one tenant during the migration.
type EarlyAdopterResult struct {
	TenantID     string `json:"tenant_id"`
	Name         string `json:"name"`
	FromTier     string `json:"from_tier"`
	ToTier       string `json:"to_tier"`
	VoucherUntil string `json:"voucher_until,omitempty"` // RFC3339; empty for the owner (no voucher) or skips
	Skipped      bool   `json:"skipped"`
	Reason       string `json:"reason,omitempty"`
}

// MigrateEarlyAdoptersRequest parameterizes a migration run.
type MigrateEarlyAdoptersRequest struct {
	VoucherTier   string `json:"voucher_tier"`    // tier granted to non-owner free tenants (default "studio")
	VoucherDays   int    `json:"voucher_days"`    // days of GrandfatherUntil access (default 365)
	OwnerTenantID string `json:"owner_tenant_id"` // optional; comped to Enterprise
	DryRun        bool   `json:"dry_run"`         // when true, compute results but write nothing
}

// MigrateEarlyAdoptersReport is the run summary returned to the admin caller and
// recorded as an audit-ledger entry.
type MigrateEarlyAdoptersReport struct {
	DryRun   bool                 `json:"dry_run"`
	Migrated int                  `json:"migrated"`
	Skipped  int                  `json:"skipped"`
	Failed   int                  `json:"failed"`
	Results  []EarlyAdopterResult `json:"results"`
}

// MigrateEarlyAdoptersUseCase moves existing hosted tenants off the `free` /
// Self-Host tier onto a real hosted tier. Non-owner free tenants get an
// early-adopter voucher (the target tier free for `voucherDays` via
// GrandfatherUntil); the owner tenant is comped to Enterprise. Idempotent:
// tenants already on a paid tier are skipped, so it's safe to re-run.
//
// Invoked on demand via the admin endpoint (not on every boot) and every run
// writes an audit-ledger summary event, so there's a persisted record of who
// ran it and what changed — no more "remember to unset the env var".
type MigrateEarlyAdoptersUseCase struct {
	tenantRepo repositories.TenantRepository
	updateSub  *UpdateSubscriptionMetadataUseCase
	auditRepo  repositories.AuditRepository
}

// NewMigrateEarlyAdoptersUseCase constructs the migration use case.
func NewMigrateEarlyAdoptersUseCase(
	tenantRepo repositories.TenantRepository,
	updateSub *UpdateSubscriptionMetadataUseCase,
	auditRepo repositories.AuditRepository,
) *MigrateEarlyAdoptersUseCase {
	return &MigrateEarlyAdoptersUseCase{tenantRepo: tenantRepo, updateSub: updateSub, auditRepo: auditRepo}
}

// hostedFreeTier reports whether a tier is a hosted-free / unassigned tier that
// should be migrated. Real paid tiers (indie/studio/scale/enterprise and the
// legacy starter/growth/team/pro aliases) are left untouched.
func hostedFreeTier(tier string) bool {
	switch strings.ToLower(strings.TrimSpace(tier)) {
	case "", defaultPlan, "self-host", "selfhost":
		return true
	default:
		return false
	}
}

// Execute migrates all active hosted-free tenants per `req`. The owner (if set)
// is comped to Enterprise; everyone else gets the voucher tier free for the
// voucher window. When req.DryRun is true nothing is written. `now` is injected
// for testability. Every run writes an audit-ledger summary event.
func (uc *MigrateEarlyAdoptersUseCase) Execute(req MigrateEarlyAdoptersRequest, now time.Time) MigrateEarlyAdoptersReport {
	voucherTier := req.VoucherTier
	if voucherTier == "" {
		voucherTier = defaultVoucherTier
	}
	voucherDays := req.VoucherDays
	if voucherDays <= 0 {
		voucherDays = defaultVoucherDays
	}

	report := MigrateEarlyAdoptersReport{DryRun: req.DryRun, Results: []EarlyAdopterResult{}}

	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("EarlyAdopterMigration: list tenants failed: %v", err)
		return report
	}

	for _, t := range tenants {
		sub := extractSubscription(t.Metadata)
		r := EarlyAdopterResult{TenantID: t.ID, Name: t.Name, FromTier: sub.Tier}

		isOwner := req.OwnerTenantID != "" && t.ID == req.OwnerTenantID
		if !isOwner && !hostedFreeTier(sub.Tier) {
			r.Skipped = true
			r.Reason = "already on a paid tier"
			report.Skipped++
			report.Results = append(report.Results, r)
			continue
		}

		newSub := &entities.SubscriptionMetadata{
			Status:          "active",
			PaymentProvider: "comp", // no real billing provider; comped/voucher
		}
		if isOwner {
			newSub.Tier = tierEnterprise
			r.ToTier = tierEnterprise
		} else {
			newSub.Tier = voucherTier
			until := now.AddDate(0, 0, voucherDays)
			newSub.GrandfatherUntil = &until
			r.ToTier = voucherTier
			r.VoucherUntil = until.Format(time.RFC3339)
		}

		if req.DryRun {
			r.Reason = "dry-run (not applied)"
			report.Migrated++ // would-migrate count
			report.Results = append(report.Results, r)
			continue
		}

		// Quotas nil → UpdateSubscriptionMetadataUseCase auto-applies the full
		// entitlement set for the new tier (events/queries/x402/retention/
		// streams/MCP scope).
		billing := &entities.TenantBillingMetadata{Subscription: newSub}
		if _, err := uc.updateSub.Execute(t.ID, billing); err != nil {
			r.Skipped = true
			r.Reason = "update failed: " + err.Error()
			report.Failed++
		} else {
			report.Migrated++
		}
		report.Results = append(report.Results, r)
	}

	uc.writeLedger(req, report, voucherTier, voucherDays)
	return report
}

// writeLedger records a persisted, queryable summary of the run.
func (uc *MigrateEarlyAdoptersUseCase) writeLedger(req MigrateEarlyAdoptersRequest, report MigrateEarlyAdoptersReport, voucherTier string, voucherDays int) {
	if uc.auditRepo == nil {
		return
	}
	event, err := entities.NewAuditEvent("billing.early_adopter_migration.run", "execute", "POST", "/admin/billing/migrate-early-adopters")
	if err != nil {
		return
	}
	event.AddMetadata("dry_run", fmt.Sprintf("%t", report.DryRun))
	event.AddMetadata("migrated", fmt.Sprintf("%d", report.Migrated))
	event.AddMetadata("skipped", fmt.Sprintf("%d", report.Skipped))
	event.AddMetadata("failed", fmt.Sprintf("%d", report.Failed))
	event.AddMetadata("voucher_tier", voucherTier)
	event.AddMetadata("voucher_days", fmt.Sprintf("%d", voucherDays))
	event.AddMetadata("owner_tenant_id", req.OwnerTenantID)
	_ = uc.auditRepo.Log(event) //nolint:errcheck // ledger logging is best-effort
}
