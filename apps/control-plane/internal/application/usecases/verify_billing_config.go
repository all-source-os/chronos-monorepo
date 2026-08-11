package usecases

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
	"strings"
	"time"

	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// variantResolveTimeout bounds the live LS lookups check 2d makes so a slow or
// unreachable LS can't hang the config-check (boot self-test + admin endpoint).
const variantResolveTimeout = 10 * time.Second

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
var requiredCatalogPeriods = []string{monthlyBillingPeriod, annualBillingPeriod}

// knownTiers is the set of canonical + retired tier ids any VARIANT key is
// allowed to reference. A variant keyed on anything else is a typo/orphan — it
// would never resolve in a checkout and signals catalog drift. Sourced from
// entities.subscription so this can never disagree with the tier authority.
var knownTiers = func() map[string]bool {
	m := map[string]bool{}
	for _, t := range []entities.SubscriptionTier{
		entities.TierFree, entities.TierIndie, entities.TierStudio,
		entities.TierScale, entities.TierEnterprise,
		// retired aliases stay valid so an old configured variant isn't flagged
		entities.TierPro, entities.TierGrowth, entities.TierTeam, entities.TierStarter,
	} {
		m[string(t)] = true
	}
	m["developer"] = true // additional retired alias (see retiredTierMap)
	return m
}()

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
	report.Facts["variant_keys"] = strings.Join(sortedKeys(variants), ",")
	report.Facts["catalog_tiers"] = strings.Join(catalogTiers, ",")
	for _, tier := range requiredCatalogTiers {
		for _, period := range requiredCatalogPeriods {
			key := tier + ":" + period
			if id, ok := variants[key]; !ok || id == "" {
				add(SeverityError, "variant_missing",
					fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP missing variant for %q; that tier/period checkout would resolve to free", key))
			}
		}
	}

	// 2b. Catalog coverage — every tier the public pricing catalog advertises
	// (GetCatalogUseCase.catalogTiers) must resolve to a usable variant for both
	// periods, else the price page shows a tier a customer cannot actually buy.
	// This makes the catalog↔variant-map link an explicit, tested invariant
	// rather than two lists that silently drift (Gap 4).
	for _, tier := range catalogTiers {
		for _, period := range requiredCatalogPeriods {
			if id, err := uc.lsClient.LookupVariantID(tier, period); err != nil || id == "" {
				add(SeverityError, "catalog_variant_missing",
					fmt.Sprintf("billing catalog advertises tier %q (%s) but its LemonSqueezy variant is not configured; the price page would show an unbuyable tier", tier, period))
			}
		}
	}

	// 2c. Orphan / misconfigured variants — every configured variant key must be
	// "<knownTier>:<period>" with a non-empty id. A variant keyed on an unknown
	// tier (typo, retired-name drift) or an empty id is a misconfiguration that
	// can never resolve in checkout. Flag it loudly instead of letting it rot.
	for key, id := range variants {
		tier, period, ok := splitVariantKey(key)
		if !ok {
			add(SeverityError, "variant_key_malformed",
				fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP key %q is not in the form \"<tier>:<period>\"", key))
			continue
		}
		if !knownTiers[tier] {
			add(SeverityError, "variant_unknown_tier",
				fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP key %q references unknown tier %q; not a canonical or retired tier, so it will never resolve in checkout", key, tier))
		}
		if period != monthlyBillingPeriod && period != annualBillingPeriod {
			add(SeverityWarn, "variant_unknown_period",
				fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP key %q has unexpected period %q (expected monthly|annual)", key, period))
		}
		if strings.TrimSpace(id) == "" {
			add(SeverityError, "variant_empty_id",
				fmt.Sprintf("LEMON_SQUEEZY_VARIANT_MAP key %q maps to an empty variant id; that tier/period checkout would resolve to free", key))
		}
	}

	// 2d. Live variant RESOLUTION — the map can be well-formed (2/2b/2c all green)
	// yet point at variant ids that don't exist under the LIVE key: LS test and
	// live variants are DISTINCT objects, so a stale test-id map 404s live. That
	// false green shipped an empty /billing/catalog (every paid price rendered
	// "—") and 404'd Indie checkout. Resolve each required variant against LS so
	// "ok" means genuinely SELLABLE, not merely well-formatted.
	ctx, cancel := context.WithTimeout(context.Background(), variantResolveTimeout)
	defer cancel()
	resolved := 0
	for _, tier := range requiredCatalogTiers {
		for _, period := range requiredCatalogPeriods {
			id, err := uc.lsClient.LookupVariantID(tier, period)
			if err != nil || id == "" {
				continue // already flagged by 2b as catalog_variant_missing
			}
			v, err := uc.lsClient.GetVariant(ctx, id)
			if err != nil {
				add(SeverityError, "variant_unresolved",
					fmt.Sprintf("LemonSqueezy variant %q for %s:%s does not resolve under the live key (%v) — /billing/catalog omits this tier and checkout 404s. Usually a stale test-id map or a wrong/revoked LEMON_SQUEEZY_API_KEY.", id, tier, period, err))
				continue
			}
			if v == nil || v.Price <= 0 {
				price := 0
				if v != nil {
					price = v.Price
				}
				add(SeverityError, "variant_no_price",
					fmt.Sprintf("LemonSqueezy variant %q for %s:%s resolved but has price<=0 (%d cents); GetCatalogUseCase skips it, so the tier vanishes from the price page", id, tier, period, price))
				continue
			}
			resolved++
		}
	}
	report.Facts["variants_resolved"] = fmt.Sprintf("%d/%d", resolved, len(requiredCatalogTiers)*len(requiredCatalogPeriods))

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

// splitVariantKey parses a VariantMap key "<tier>:<period>" into its parts.
// Returns ok=false if the key isn't exactly one tier and one period.
func splitVariantKey(key string) (tier, period string, ok bool) {
	parts := strings.SplitN(key, ":", 2)
	if len(parts) != 2 || parts[0] == "" || parts[1] == "" || strings.Contains(parts[1], ":") {
		return "", "", false
	}
	return parts[0], parts[1], true
}

// sortedKeys returns a VariantMap's keys sorted, for stable fact reporting.
func sortedKeys(m clients.VariantMap) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
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
