package http //nolint:revive // same-package tests

import (
	"encoding/json"
	"testing"
)

func TestBillingLinks(t *testing.T) {
	links := billingLinks("t-123")

	if links["self"].Href != "/api/v1/billing/checkout" {
		t.Errorf("self link = %q, want /api/v1/billing/checkout", links["self"].Href)
	}
	if links["portal"].Href != "/api/v1/billing/portal" {
		t.Errorf("portal link = %q, want /api/v1/billing/portal", links["portal"].Href)
	}
	if links["overage"].Href != "/api/v1/billing/overage?tenant_id=t-123" {
		t.Errorf("overage link = %q, want /api/v1/billing/overage?tenant_id=t-123", links["overage"].Href)
	}
	if links["projected-charges"].Href != "/api/v1/billing/projected-charges?tenant_id=t-123" {
		t.Errorf("projected-charges link = %q", links["projected-charges"].Href)
	}
	if links["tenant"].Href != "/api/v1/tenants/t-123" {
		t.Errorf("tenant link = %q, want /api/v1/tenants/t-123", links["tenant"].Href)
	}
}

func TestOverageLinks(t *testing.T) {
	links := overageLinks("t-456")

	if links["self"].Href != "/api/v1/billing/overage?tenant_id=t-456" {
		t.Errorf("self link = %q", links["self"].Href)
	}
	if links["enable"].Href != "/api/v1/billing/overage/enable" {
		t.Errorf("enable link = %q", links["enable"].Href)
	}
	if links["disable"].Href != "/api/v1/billing/overage/disable" {
		t.Errorf("disable link = %q", links["disable"].Href)
	}
	if links["tenant"].Href != "/api/v1/tenants/t-456" {
		t.Errorf("tenant link = %q", links["tenant"].Href)
	}
}

func TestProjectedChargesLinks(t *testing.T) {
	links := projectedChargesLinks("t-789")

	if links["self"].Href != "/api/v1/billing/projected-charges?tenant_id=t-789" {
		t.Errorf("self link = %q", links["self"].Href)
	}
	if links["overage"].Href != "/api/v1/billing/overage?tenant_id=t-789" {
		t.Errorf("overage link = %q", links["overage"].Href)
	}
	if links["tenant"].Href != "/api/v1/tenants/t-789" {
		t.Errorf("tenant link = %q", links["tenant"].Href)
	}
}

func TestCheckoutHALResponseJSON(t *testing.T) {
	resp := checkoutHALResponse{
		HALResource: HALResource{Links: billingLinks("t-123")},
	}

	data, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links, ok := parsed["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in JSON output")
	}

	selfLink, ok := links["self"].(map[string]interface{})
	if !ok {
		t.Fatal("self link not found")
	}
	if selfLink["href"] != "/api/v1/billing/checkout" {
		t.Errorf("self.href = %v", selfLink["href"])
	}
}

func TestOverageHALResponseJSON(t *testing.T) {
	resp := overageHALResponse{
		HALResource: HALResource{Links: overageLinks("t-123")},
	}

	data, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("marshal error: %v", err)
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("unmarshal error: %v", err)
	}

	links, ok := parsed["_links"].(map[string]interface{})
	if !ok {
		t.Fatal("_links not found in JSON output")
	}

	// Check enable/disable links are present
	if _, ok := links["enable"]; !ok {
		t.Error("enable link not found")
	}
	if _, ok := links["disable"]; !ok {
		t.Error("disable link not found")
	}
}
