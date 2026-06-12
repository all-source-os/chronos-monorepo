package usecases

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Severity levels for a billing config issue.
const (
	SeverityError = "error"
	SeverityWarn  = "warn"
)

// lemonSqueezySecretMaxLen is the LemonSqueezy dashboard signing-secret field
// cap. A secret longer than this is silently truncated on the LS side, so the
// dashboard secret can never equal a >40-char Fly secret → every webhook 400s.
// This was a root cause of the stuck-on-Indie saga.
const lemonSqueezySecretMaxLen = 40

// requiredCatalogTiers are the live paid tiers that MUST have a configured
// variant for both billing periods. enterprise is sales-led (mailto, no
// variant) and self-host is free, so neither needs a catalog variant.
var requiredCatalogTiers = []string{"indie", "studio", "scale"}

// requiredCatalogPeriods are the billing periods every paid tier must offer.
var requiredCatalogPeriods = []string{"monthly", "annual"}

// BillingConfigIssue is a single problem found while verifying billing config.
type BillingConfigIssue struct {
	Severity string `json:"severity"` // "error" | "warn"
	Code     string `json:"code"`
	Message  string `json:"message"`
}

// BillingConfigReport is the result of a billing config verification pass.
type BillingConfigReport struct {
	OK      bool                 `json:"ok"` // false if any error-severity issue
	Issues  []BillingConfigIssue `json:"issues"`
	Facts   map[string]string    `json:"facts"`   // human-readable summary of what was checked
	Manual  []string             `json:"manual"`  // checks that can't be automated
	Skipped bool                 `json:"skipped"` // true when LS billing isn't configured at all
}

// VerifyBillingConfigUseCase asserts that the LemonSqueezy billing wiring is
// internally consistent at deploy time and on demand, turning silent
// misconfigurations (the recurring footgun) into loud, visible failures.
//
// It verifies what IS machine-checkable: variant-map coverage, webhook-secret
// presence + length, and store/key presence. It cannot verify that the LS
// dashboard signing secret EQUALS the Fly secret — LS never returns the secret
// value — so that one check is surfaced under Manual with the reconcile recipe.
type VerifyBillingConfigUseCase struct {
	lsClient      clients.LemonSqueezyClient
	webhookSecret string
}

// NewVerifyBillingConfigUseCase constructs the verifier. lsClient may be nil
// (billing not configured) — Execute then reports Skipped.
func NewVerifyBillingConfigUseCase(lsClient clients.LemonSqueezyClient, webhookSecret string) *VerifyBillingConfigUseCase {
	return &VerifyBillingConfigUseCase{lsClient: lsClient, webhookSecret: webhookSecret}
}

// Execute runs every check and returns a structured report.
func (uc *VerifyBillingConfigUseCase) Execute() BillingConfigReport {
	report := BillingConfigReport{
		OK:    true,
		Facts: map[string]string{},
	}

	if uc.lsClient == nil {
		report.Skipped = true
		report.Facts["billing"] = "not configured (no LemonSqueezy client)"
		return report
	}

	add := func(sev, code, msg string) {
		report.Issues = append(report.Issues, BillingConfigIssue{Severity: sev, Code: code, Message: msg})
		if sev == SeverityError {
			report.OK = false
		}
	}

	// 1. Store ID present.
	if uc.lsClient.GetStoreID() == "" {
		add(SeverityError, "store_missing", "LEMON_SQUEEZY_STORE_ID is empty; checkout cannot create orders")
	} else {
		report.Facts["store_id"] = uc.lsClient.GetStoreID()
	}

	// 2. Variant-map coverage — every paid tier × period must resolve to a
	// non-empty variant ID, else a paid checkout silently falls back to free.
	variants := uc.lsClient.VariantMap()
	report.Facts["variant_count"] = fmt.Sprintf("%d", len(variants))
	for _, tier := range requiredCatalogTiers {
		for _, period := range requiredCatalogPeriods {
			key := tier + ":" + period
			if id, ok := variants[key]; !ok || id == "" {
				add(SeverityError, "variant_missing",
					fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP missing variant for %q; that tier/period checkout would resolve to free", key))
			}
		}
	}

	// 3. Webhook secret present + within the LS dashboard length cap.
	switch n := len(uc.webhookSecret); {
	case n == 0:
		add(SeverityError, "webhook_secret_missing",
			"LEMON_SQUEEZY_WEBHOOK_SECRET is empty; every webhook delivery 400s on signature, so tiers never update")
	case n > lemonSqueezySecretMaxLen:
		add(SeverityError, "webhook_secret_too_long",
			fmt.Sprintf("LEMON_SQUEEZY_WEBHOOK_SECRET is %d chars; LS caps the signing-secret field at %d, so the dashboard secret is silently truncated and can never match → every webhook 400s",
				n, lemonSqueezySecretMaxLen))
	default:
		report.Facts["webhook_secret_len"] = fmt.Sprintf("%d", n)
		// Sanity: the configured secret can actually produce the HMAC the
		// handler verifies with (catches a wrong key type / encoding bug).
		if !hmacSelfTest(uc.webhookSecret) {
			add(SeverityError, "webhook_secret_hmac_failed", "HMAC self-test with the configured webhook secret failed")
		}
	}

	// Surface the one check that cannot be automated: LS returns the webhook
	// secret as write-only, so equality with the dashboard can't be read back.
	report.Manual = append(report.Manual,
		"LS dashboard signing secret must equal LEMON_SQUEEZY_WEBHOOK_SECRET (LS never returns the value to compare). To reconcile: PATCH /v1/webhooks/{id} {data.attributes.secret} = the Fly secret. See docs/runbooks/PRICING_BILLING_CUTOVER.md.")

	return report
}

// hmacSelfTest confirms the secret produces a stable HMAC-SHA256 hex digest the
// same way the webhook handler computes it.
func hmacSelfTest(secret string) bool {
	const probe = "billing-config-self-test"
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write([]byte(probe))
	sum := hex.EncodeToString(mac.Sum(nil))
	return len(sum) == sha256.Size*2
}
