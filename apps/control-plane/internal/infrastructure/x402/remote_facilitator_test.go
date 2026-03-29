package x402_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/x402"
)

// newTestRemoteFacilitator creates a RemoteFacilitator pointing at a test HTTP server.
func newTestRemoteFacilitator(t *testing.T, handler http.HandlerFunc) (*x402.RemoteFacilitator, *httptest.Server) {
	t.Helper()
	srv := httptest.NewServer(handler)
	t.Cleanup(srv.Close)
	return x402.NewRemoteFacilitator(srv.URL), srv
}

// minPayment returns minimal valid test fixtures for facilitator calls.
func minPayment() (*x402.PaymentPayload, *x402.PaymentRequired) {
	payment := &x402.PaymentPayload{X402Version: 2}
	requirements := &x402.PaymentRequired{X402Version: 2}
	return payment, requirements
}

func TestRemoteFacilitator_Verify_HappyPath(t *testing.T) {
	want := x402.VerifyResponse{IsValid: true, Payer: "0xabc"}
	f, _ := newTestRemoteFacilitator(t, func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/verify" {
			t.Errorf("unexpected path %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(want); err != nil {
			t.Errorf("encode response: %v", err)
		}
	})

	payment, requirements := minPayment()
	got, err := f.Verify(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !got.IsValid {
		t.Errorf("IsValid = false, want true")
	}
	if got.Payer != want.Payer {
		t.Errorf("Payer = %q, want %q", got.Payer, want.Payer)
	}
}

func TestRemoteFacilitator_Verify_Non2xx(t *testing.T) {
	f, _ := newTestRemoteFacilitator(t, func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "internal server error", http.StatusInternalServerError)
	})

	payment, requirements := minPayment()
	_, err := f.Verify(context.Background(), payment, requirements)
	if err == nil {
		t.Fatal("expected error for 500 response, got nil")
	}
	if !strings.Contains(err.Error(), "500") {
		t.Errorf("error should mention status code 500, got: %v", err)
	}
}

func TestRemoteFacilitator_Settle_HappyPath(t *testing.T) {
	want := x402.SettleResponse{Success: true, Transaction: "0xtxhash", Network: "eip155:8453"}
	f, _ := newTestRemoteFacilitator(t, func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/settle" {
			t.Errorf("unexpected path %s", r.URL.Path)
		}
		w.Header().Set("Content-Type", "application/json")
		if err := json.NewEncoder(w).Encode(want); err != nil {
			t.Errorf("encode response: %v", err)
		}
	})

	payment, requirements := minPayment()
	got, err := f.Settle(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !got.Success {
		t.Errorf("Success = false, want true")
	}
	if got.Transaction != want.Transaction {
		t.Errorf("Transaction = %q, want %q", got.Transaction, want.Transaction)
	}
}

func TestRemoteFacilitator_Settle_Non2xx(t *testing.T) {
	f, _ := newTestRemoteFacilitator(t, func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadRequest)
		if _, err := w.Write([]byte(`{"error":"invalid payment"}`)); err != nil {
			t.Errorf("write response: %v", err)
		}
	})

	payment, requirements := minPayment()
	_, err := f.Settle(context.Background(), payment, requirements)
	if err == nil {
		t.Fatal("expected error for 400 response, got nil")
	}
	if !strings.Contains(err.Error(), "400") {
		t.Errorf("error should mention status code 400, got: %v", err)
	}
}

func TestRemoteFacilitator_Timeout(t *testing.T) {
	f, _ := newTestRemoteFacilitator(t, func(w http.ResponseWriter, r *http.Request) {
		// Sleep longer than the context deadline.
		select {
		case <-r.Context().Done():
		case <-time.After(200 * time.Millisecond):
		}
		w.WriteHeader(http.StatusOK)
	})

	// Override the internal HTTP client with a 50ms timeout.
	fastF := x402.NewRemoteFacilitatorWithTimeout(f.BaseURL(), 50*time.Millisecond)

	payment, requirements := minPayment()
	_, err := fastF.Verify(context.Background(), payment, requirements)
	if err == nil {
		t.Fatal("expected timeout error, got nil")
	}
}
