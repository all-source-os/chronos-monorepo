package clients

import (
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"net/url"
	"testing"
)

func TestStripeCreateCheckoutSession_Success(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/v1/checkout/sessions" {
			t.Errorf("unexpected request: %s %s", r.Method, r.URL.Path)
		}
		if r.Header.Get("Authorization") != "Bearer sk_test_key" {
			t.Errorf("missing or wrong auth header: %s", r.Header.Get("Authorization"))
		}

		body, _ := io.ReadAll(r.Body)             //nolint:errcheck
		params, _ := url.ParseQuery(string(body)) //nolint:errcheck // test handler

		if params.Get("mode") != "subscription" {
			t.Errorf("expected mode=subscription, got %s", params.Get("mode"))
		}
		if params.Get("line_items[0][price]") != "price_pro" {
			t.Errorf("expected price_pro, got %s", params.Get("line_items[0][price]"))
		}
		if params.Get("metadata[tenant_id]") != "tenant-123" {
			t.Errorf("expected tenant-123 in metadata, got %s", params.Get("metadata[tenant_id]"))
		}
		if params.Get("subscription_data[metadata][tenant_id]") != "tenant-123" {
			t.Errorf("expected tenant-123 in subscription_data metadata")
		}
		if params.Get("customer_email") != "user@example.com" {
			t.Errorf("expected customer_email user@example.com, got %s", params.Get("customer_email"))
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]string{ //nolint:errcheck // test handler
			"id":  "cs_test_123",
			"url": "https://checkout.stripe.com/c/pay/cs_test_123",
		})
	}))
	defer server.Close()

	client := newStripeClient(server.URL, "sk_test_key", StripePriceMap{"pro": "price_pro"})

	result, err := client.CreateCheckoutSession(context.Background(), StripeCheckoutRequest{
		PriceID:       "price_pro",
		CustomerEmail: "user@example.com",
		Metadata:      map[string]string{"tenant_id": "tenant-123"},
		SuccessURL:    "https://app.example.com/success",
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.ID != "cs_test_123" {
		t.Errorf("expected ID cs_test_123, got %s", result.ID)
	}
	if result.URL != "https://checkout.stripe.com/c/pay/cs_test_123" {
		t.Errorf("unexpected URL: %s", result.URL)
	}
}

func TestStripeCreateCheckoutSession_APIError(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"error":{"message":"Invalid price"}}`)) //nolint:errcheck // test handler
	}))
	defer server.Close()

	client := newStripeClient(server.URL, "sk_test_key", nil)

	_, err := client.CreateCheckoutSession(context.Background(), StripeCheckoutRequest{
		PriceID:    "bad_price",
		SuccessURL: "https://example.com/success",
	})
	if err == nil {
		t.Fatal("expected error, got nil")
	}
}

func TestStripeGetCustomerPortalURL_Success(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/v1/billing_portal/sessions" {
			t.Errorf("unexpected request: %s %s", r.Method, r.URL.Path)
		}

		body, _ := io.ReadAll(r.Body)             //nolint:errcheck
		params, _ := url.ParseQuery(string(body)) //nolint:errcheck // test handler
		if params.Get("customer") != "cus_123" {
			t.Errorf("expected customer=cus_123, got %s", params.Get("customer"))
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]string{ //nolint:errcheck // test handler
			"url": "https://billing.stripe.com/p/session/test_portal",
		})
	}))
	defer server.Close()

	client := newStripeClient(server.URL, "sk_test_key", nil)

	portalURL, err := client.GetCustomerPortalURL(context.Background(), "cus_123", "")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if portalURL != "https://billing.stripe.com/p/session/test_portal" {
		t.Errorf("unexpected portal URL: %s", portalURL)
	}
}

func TestStripeGetSubscription_Success(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/v1/subscriptions/sub_test_1" {
			t.Errorf("unexpected request: %s %s", r.Method, r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]interface{}{ //nolint:errcheck // test handler
			"id":                   "sub_test_1",
			"status":               "active",
			"customer":             "cus_123",
			"current_period_end":   1700000000,
			"cancel_at_period_end": false,
			"metadata":             map[string]string{"tenant_id": "t-1"},
		})
	}))
	defer server.Close()

	client := newStripeClient(server.URL, "sk_test_key", nil)

	sub, err := client.GetSubscription(context.Background(), "sub_test_1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if sub.ID != "sub_test_1" {
		t.Errorf("expected ID sub_test_1, got %s", sub.ID)
	}
	if sub.Status != "active" {
		t.Errorf("expected status active, got %s", sub.Status)
	}
	if sub.CustomerID != "cus_123" {
		t.Errorf("expected customer cus_123, got %s", sub.CustomerID)
	}
}

func TestStripeGetSubscription_NotFound(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
		_, _ = w.Write([]byte(`{"error":{"message":"No such subscription"}}`)) //nolint:errcheck // test handler
	}))
	defer server.Close()

	client := newStripeClient(server.URL, "sk_test_key", nil)

	_, err := client.GetSubscription(context.Background(), "nonexistent")
	if err == nil {
		t.Fatal("expected error, got nil")
	}
}

func TestStripeLookupPriceID(t *testing.T) {
	client := newStripeClient("http://unused", "key", StripePriceMap{
		"free": "price_free",
		"pro":  "price_pro",
		"team": "price_team",
	})

	tests := []struct {
		tier    string
		wantID  string
		wantErr bool
	}{
		{"free", "price_free", false},
		{"pro", "price_pro", false},
		{"team", "price_team", false},
		{"Pro", "price_pro", false},   // case-insensitive
		{"TEAM", "price_team", false}, // case-insensitive
		{"unknown", "", true},
	}

	for _, tt := range tests {
		id, err := client.LookupPriceID(tt.tier)
		if (err != nil) != tt.wantErr {
			t.Errorf("LookupPriceID(%q): err=%v, wantErr=%v", tt.tier, err, tt.wantErr)
		}
		if id != tt.wantID {
			t.Errorf("LookupPriceID(%q): got %q, want %q", tt.tier, id, tt.wantID)
		}
	}
}

func TestStripeLookupPriceID_NoMap(t *testing.T) {
	client := newStripeClient("http://unused", "key", nil)

	_, err := client.LookupPriceID("free")
	if err == nil {
		t.Fatal("expected error when price map is nil")
	}
}

func TestNewStripeClient_EnvVars(t *testing.T) {
	// Missing secret key
	t.Setenv("STRIPE_SECRET_KEY", "")
	_, err := NewStripeClient()
	if err == nil {
		t.Fatal("expected error for missing secret key")
	}

	// Valid config
	t.Setenv("STRIPE_SECRET_KEY", "sk_test_123")
	t.Setenv("STRIPE_PRICE_MAP", `{"free":"price_1","pro":"price_2"}`)
	client, err := NewStripeClient()
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if client == nil {
		t.Fatal("expected non-nil client")
	}

	// Invalid price map JSON
	t.Setenv("STRIPE_PRICE_MAP", `{invalid}`)
	_, err = NewStripeClient()
	if err == nil {
		t.Fatal("expected error for invalid price map JSON")
	}
}
