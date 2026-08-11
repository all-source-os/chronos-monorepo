package usecases

import (
	"context"

	"github.com/allsource/control-plane/internal/domain/entities"
)

// EditionDiagnosis is the read-only result of the edition-trap detector
// (§4.2 fix_edition_trap, diagnose-only by default).
type EditionDiagnosis struct {
	TrapDetected            bool   `json:"trap_detected"`
	QSEdition               string `json:"qs_edition"` // "community" | "enterprise" | "" (unknown — QS /health may not expose it)
	NonCommunityDataTenants int    `json:"non_community_data_tenants"`
	Threshold               int    `json:"threshold"`
	// RemediationCommand is the exact operator-executed command. The env mutation
	// is NOT done by the API (it re-routes the WHOLE fleet); the operator runs it.
	RemediationCommand string `json:"remediation_command"`
	// VerifyCommand confirms a NEW Fly release was created (runbook trap #4: a
	// `fly deploy` that creates no release is a no-op).
	VerifyCommand string `json:"verify_command"`
	Explanation   string `json:"explanation"`
}

// DiagnoseEdition detects the edition trap and returns the exact remediation. It
// never mutates anything — the env switch is fleet-wide and operator-executed.
func (uc *FleetHealthUseCase) DiagnoseEdition(ctx context.Context) *EditionDiagnosis {
	detected, edition, n := uc.EditionTrapDetected(ctx)
	d := &EditionDiagnosis{
		TrapDetected:            detected,
		QSEdition:               edition,
		NonCommunityDataTenants: n,
		Threshold:               uc.editionTrapThreshold,
		RemediationCommand:      "fly secrets set ALLSOURCE_EDITION=enterprise -a allsource-query",
		VerifyCommand:           "fly releases -a allsource-query  # confirm a NEW release was created (a deploy with no release is a no-op)",
	}
	switch {
	case detected:
		d.Explanation = "Query Service edition is 'community', which pins EVERY request to the 'community' tenant. " +
			"The fleet has " + itoaInt(n) + " non-community tenants WITH data, so real tenants' dashboards read empty. " +
			"Set ALLSOURCE_EDITION=enterprise on allsource-query and confirm a new Fly release."
	case edition == editionCommunity:
		d.Explanation = "Query Service edition is 'community' but fewer than the threshold non-community data tenants exist; " +
			"not flagged as the trap yet, but switch to enterprise before onboarding paying tenants."
	case edition == "":
		d.Explanation = "Could not read the QS edition from /health (the endpoint may not expose it). " +
			"Verify ALLSOURCE_EDITION=enterprise on allsource-query manually if dashboards read empty despite synced data."
	default:
		d.Explanation = "Query Service edition is '" + edition + "' — no edition trap."
	}
	return d
}

// IdentityDiagnosis is the read-only result of the tenant/JWT/store divergence
// check (§4.2 diagnose_tenant_identity). NO mutation — the runbook fix is a
// re-login, never a data move.
type IdentityDiagnosis struct {
	RequestedTenantID string `json:"requested_tenant_id"`     // the tenant the caller asked about (path :id)
	JWTTenantID       string `json:"jwt_tenant_id,omitempty"` // the tenant the caller's admin JWT claims
	StoreTenantExists bool   `json:"store_tenant_exists"`     // does a tenant with this id exist in Core?
	StoreHasData      bool   `json:"store_has_data"`          // does that tenant have events in Core?
	Diverged          bool   `json:"diverged"`                // JWT tenant != requested tenant
	ExpectedSlug      string `json:"expected_slug,omitempty"` // TenantSlug(email) — who SHOULD own this data
	Fix               string `json:"fix"`
	Explanation       string `json:"explanation"`
}

// DiagnoseIdentity compares (a) the tenant the caller asked about, (b) the tenant
// the caller's JWT claims, and (c) what the store holds, and returns the
// divergence + the re-login fix. jwtTenantID/jwtEmail come from the admin auth
// context. No mutation (auto-merging tenants is explicitly out of scope).
func (uc *FleetHealthUseCase) DiagnoseIdentity(ctx context.Context, requestedTenantID, jwtTenantID, jwtEmail string) *IdentityDiagnosis {
	d := &IdentityDiagnosis{
		RequestedTenantID: requestedTenantID,
		JWTTenantID:       jwtTenantID,
	}

	// (c) what the store holds for the requested tenant
	if t, err := uc.tenantRepo.FindByID(requestedTenantID); err == nil && t != nil {
		d.StoreTenantExists = true
		if stats := uc.tenantStats(ctx, requestedTenantID); stats != nil && stats.EventCount > 0 {
			d.StoreHasData = true
		}
	}

	if jwtEmail != "" {
		d.ExpectedSlug = entities.TenantSlug(jwtEmail)
	}

	d.Diverged = jwtTenantID != "" && jwtTenantID != requestedTenantID

	switch {
	case d.Diverged:
		d.Fix = "Re-login with the account whose TenantSlug(email) equals the data tenant (" + requestedTenantID + "). " +
			"Do NOT move data between tenants."
		d.Explanation = "The caller's JWT claims tenant '" + jwtTenantID + "' but is asking about '" + requestedTenantID + "'. " +
			"This is the wrong-tenant / JWT-mismatch fingerprint: the dashboard reads the JWT's tenant, which has no data, " +
			"while the real data sits under '" + requestedTenantID + "'."
	case !d.StoreTenantExists:
		d.Fix = "No tenant '" + requestedTenantID + "' exists in Core. Confirm the id; a silent auto-provision may have created a different tenant from the login email (expected slug: " + d.ExpectedSlug + ")."
		d.Explanation = "The requested tenant does not exist in the store. If data was 'lost', it is almost certainly under a different tenant id (wrong-tenant auto-provision), not deleted — Core is durable."
	case d.StoreTenantExists && !d.StoreHasData:
		d.Fix = "Tenant exists but has no events. If the user expected data, check whether they ingested under a different tenant id (re-login as the correct account)."
		d.Explanation = "Tenant exists with zero events. Empty reads here are a read-path/identity symptom, never data loss."
	default:
		d.Fix = "No identity divergence detected."
		d.Explanation = "The JWT tenant matches the requested tenant and the store holds its data."
	}
	return d
}
