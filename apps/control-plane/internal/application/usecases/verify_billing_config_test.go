package usecases

import (
	"strings"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

type verifyMockLS struct {
	clients.LemonSqueezyClient
	variants clients.VariantMap
	storeID  string
}

func (m *verifyMockLS) VariantMap() clients.VariantMap { return m.variants }
func (m *verifyMockLS) GetStoreID() string             { return m.storeID }

// LookupVariantID resolves tier+period against the configured variant map,
// mirroring the real client's catalog lookup so the catalog-coverage check has a
// faithful backing in tests. Empty period defaults to monthly.
func (m *verifyMockLS) LookupVariantID(tier, period string) (string, error) {
	if period == "" {
		period = "monthly"
	}
	if id, ok := m.variants[tier+":"+period]; ok {
		return id, nil
	}
	return "", nil
}

func fullVariants() clients.VariantMap {
	return clients.VariantMap{
		"indie:monthly":  "1",
		"indie:annual":   "2",
		"studio:monthly": "3",
		"studio:annual":  "4",
		"scale:monthly":  "5",
		"scale:annual":   "6",
	}
}

func hasCode(r BillingConfigReport, code string) bool {
	for _, i := range r.Issues {
		if i.Code == code {
			return true
		}
	}
	return false
}

func TestVerifyBillingConfig_Skipped(t *testing.T) {
	r := NewVerifyBillingConfigUseCase(nil, "secret").Execute()
	if !r.Skipped || !r.OK {
		t.Fatalf("nil client should skip+ok, got %+v", r)
	}
}

func TestVerifyBillingConfig_AllGood(t *testing.T) {
	ls := &verifyMockLS{variants: fullVariants(), storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "abc123def456").Execute()
	if !r.OK || len(r.Issues) != 0 {
		t.Fatalf("expected clean report, got %+v", r)
	}
	if len(r.Manual) == 0 {
		t.Errorf("expected the secret-equality manual check to be surfaced")
	}
}

func TestVerifyBillingConfig_MissingVariant(t *testing.T) {
	v := fullVariants()
	delete(v, "studio:annual")
	v["scale:monthly"] = "" // present-but-empty also counts as missing
	ls := &verifyMockLS{variants: v, storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "secret").Execute()
	if r.OK || !hasCode(r, "variant_missing") {
		t.Fatalf("expected variant_missing error, got %+v", r)
	}
}

func TestVerifyBillingConfig_WebhookSecret(t *testing.T) {
	ls := &verifyMockLS{variants: fullVariants(), storeID: "store-1"}

	if r := NewVerifyBillingConfigUseCase(ls, "").Execute(); r.OK || !hasCode(r, "webhook_secret_missing") {
		t.Errorf("empty secret should error, got %+v", r)
	}

	long := strings.Repeat("x", lemonSqueezySecretMaxLen+1)
	if r := NewVerifyBillingConfigUseCase(ls, long).Execute(); r.OK || !hasCode(r, "webhook_secret_too_long") {
		t.Errorf("over-length secret should error, got %+v", r)
	}
}

func TestVerifyBillingConfig_MissingStore(t *testing.T) {
	ls := &verifyMockLS{variants: fullVariants(), storeID: ""}
	r := NewVerifyBillingConfigUseCase(ls, "secret").Execute()
	if r.OK || !hasCode(r, "store_missing") {
		t.Fatalf("expected store_missing error, got %+v", r)
	}
}

// Gap-4 guard: a catalog tier whose LemonSqueezy variant is missing must be
// flagged. Dropping studio:monthly leaves the catalog advertising a tier nobody
// can buy → catalog_variant_missing (and also the existing variant_missing).
func TestVerifyBillingConfig_CatalogTierMissingVariant(t *testing.T) {
	v := fullVariants()
	delete(v, "studio:monthly")
	ls := &verifyMockLS{variants: v, storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "abc123def456").Execute()
	if r.OK || !hasCode(r, "catalog_variant_missing") {
		t.Fatalf("expected catalog_variant_missing error for studio:monthly, got %+v", r)
	}
}

// A variant keyed on a tier outside the canonical/retired set is a
// misconfiguration (typo / drift) and must be flagged.
func TestVerifyBillingConfig_OrphanVariantTier(t *testing.T) {
	v := fullVariants()
	v["enterprize:monthly"] = "999" // typo'd tier (enterprize) — not a known tier
	ls := &verifyMockLS{variants: v, storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "abc123def456").Execute()
	if r.OK || !hasCode(r, "variant_unknown_tier") {
		t.Fatalf("expected variant_unknown_tier error for enterprize, got %+v", r)
	}
}

// A retired-but-valid tier (pro) configured with a real id must NOT be flagged
// as unknown — old configured variants are allowed to linger.
func TestVerifyBillingConfig_RetiredTierVariantAllowed(t *testing.T) {
	v := fullVariants()
	v["pro:monthly"] = "777" // retired alias, still resolvable
	ls := &verifyMockLS{variants: v, storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "abc123def456").Execute()
	if hasCode(r, "variant_unknown_tier") {
		t.Fatalf("retired tier pro should not be flagged unknown, got %+v", r)
	}
	if !r.OK {
		t.Fatalf("a fully-configured catalog + a retired alias should be OK, got %+v", r)
	}
}

// An empty variant id (present-but-blank) for a known tier must be flagged.
func TestVerifyBillingConfig_EmptyVariantID(t *testing.T) {
	v := fullVariants()
	v["scale:annual"] = "" // configured but blank
	ls := &verifyMockLS{variants: v, storeID: "store-1"}
	r := NewVerifyBillingConfigUseCase(ls, "abc123def456").Execute()
	if r.OK || !hasCode(r, "variant_empty_id") {
		t.Fatalf("expected variant_empty_id error for scale:annual, got %+v", r)
	}
}
