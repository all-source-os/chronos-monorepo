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
