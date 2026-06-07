package usecases

import (
	"log"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/domain/repositories"
)

// EarlyAdopterResult records what happened to one tenant during the migration.
type EarlyAdopterResult struct {
	TenantID     string
	Name         string
	FromTier     string
	ToTier       string
	VoucherUntil string // RFC3339; empty for the owner (no voucher) or skips
	Skipped      bool
	Reason       string
}

// MigrateEarlyAdoptersUseCase moves existing hosted tenants off the `free` /
// Self-Host tier onto a real hosted tier. Non-owner free tenants get an
// early-adopter voucher (the target tier free for `voucherDays` via
// GrandfatherUntil); the owner tenant is comped to Enterprise. Idempotent:
// tenants already on a paid tier are skipped, so it's safe to re-run.
type MigrateEarlyAdoptersUseCase struct {
	tenantRepo repositories.TenantRepository
	updateSub  *UpdateSubscriptionMetadataUseCase
}

// NewMigrateEarlyAdoptersUseCase constructs the migration use case.
func NewMigrateEarlyAdoptersUseCase(
	tenantRepo repositories.TenantRepository,
	updateSub *UpdateSubscriptionMetadataUseCase,
) *MigrateEarlyAdoptersUseCase {
	return &MigrateEarlyAdoptersUseCase{tenantRepo: tenantRepo, updateSub: updateSub}
}

// hostedFreeTier reports whether a tier is a hosted-free / unassigned tier that
// should be migrated. Real paid tiers (indie/studio/scale/enterprise and the
// legacy starter/growth/team/pro aliases) are left untouched.
func hostedFreeTier(tier string) bool {
	switch strings.ToLower(strings.TrimSpace(tier)) {
	case "", "free", "self-host", "selfhost":
		return true
	default:
		return false
	}
}

// Execute migrates all active hosted-free tenants. `ownerTenantID`, when set,
// is comped to Enterprise; everyone else gets `voucherTier` free for
// `voucherDays`. `now` is injected for testability.
func (uc *MigrateEarlyAdoptersUseCase) Execute(
	voucherTier string,
	voucherDays int,
	ownerTenantID string,
	now time.Time,
) []EarlyAdopterResult {
	tenants, err := uc.tenantRepo.FindActive()
	if err != nil {
		log.Printf("EarlyAdopterMigration: list tenants failed: %v", err)
		return nil
	}

	results := make([]EarlyAdopterResult, 0, len(tenants))
	for _, t := range tenants {
		sub := extractSubscription(t.Metadata)
		r := EarlyAdopterResult{TenantID: t.ID, Name: t.Name, FromTier: sub.Tier}

		isOwner := ownerTenantID != "" && t.ID == ownerTenantID
		if !isOwner && !hostedFreeTier(sub.Tier) {
			r.Skipped = true
			r.Reason = "already on a paid tier"
			results = append(results, r)
			continue
		}

		newSub := &entities.SubscriptionMetadata{
			Status:          "active",
			PaymentProvider: "comp", // no real billing provider; comped/voucher
		}
		if isOwner {
			newSub.Tier = "enterprise"
			r.ToTier = "enterprise"
		} else {
			newSub.Tier = voucherTier
			until := now.AddDate(0, 0, voucherDays)
			newSub.GrandfatherUntil = &until
			r.ToTier = voucherTier
			r.VoucherUntil = until.Format(time.RFC3339)
		}

		// Quotas nil → UpdateSubscriptionMetadataUseCase auto-applies the full
		// entitlement set for the new tier (events/queries/x402/retention/
		// streams/MCP scope).
		billing := &entities.TenantBillingMetadata{Subscription: newSub}
		if _, err := uc.updateSub.Execute(t.ID, billing); err != nil {
			r.Skipped = true
			r.Reason = "update failed: " + err.Error()
		}
		results = append(results, r)
	}
	return results
}
