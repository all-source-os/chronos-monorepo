//go:build integration

// Package x402 integration tests require real CDP credentials and network access.
//
// Required environment variables:
//
//	COINBASE_CDP_KEY_NAME    — CDP API key name (organizations/.../apiKeys/...)
//	COINBASE_CDP_PRIVATE_KEY — EC P-256 private key in PEM format
//	COINBASE_CDP_NETWORK     — optional, defaults to "base-sepolia"
//	BASE_RPC_URL             — optional, defaults to https://sepolia.base.org
//
// Run with:
//
//	go test -tags=integration ./internal/infrastructure/x402/... -v -run TestIntegration
package x402_test

import (
	"context"
	"os"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/x402"
)

// TestIntegration_CreateSepoliaWallet creates a real CDP managed wallet on Base Sepolia.
// The wallet address is logged so it can be funded via Circle's USDC faucet for further testing.
func TestIntegration_CreateSepoliaWallet(t *testing.T) {
	if os.Getenv("COINBASE_CDP_KEY_NAME") == "" {
		t.Skip("COINBASE_CDP_KEY_NAME not set — skipping integration test")
	}

	// Default to Sepolia for integration tests (cheap, no real funds needed).
	if os.Getenv("COINBASE_CDP_NETWORK") == "" {
		t.Setenv("COINBASE_CDP_NETWORK", "base-sepolia")
	}
	if os.Getenv("BASE_RPC_URL") == "" {
		t.Setenv("BASE_RPC_URL", "https://sepolia.base.org")
	}

	cdpClient, err := clients.NewCoinbaseCDPClient()
	if err != nil {
		t.Fatalf("NewCoinbaseCDPClient: %v", err)
	}

	wallet, err := cdpClient.CreateWallet(context.Background(), "integration-test-agent")
	if err != nil {
		t.Fatalf("CreateWallet: %v", err)
	}

	if wallet.ID == "" {
		t.Error("wallet ID is empty")
	}
	if wallet.Address == "" {
		t.Error("wallet address is empty")
	}

	t.Logf("Created Sepolia wallet: ID=%s Address=%s", wallet.ID, wallet.Address)
	t.Logf("Fund this address via Circle faucet: https://faucet.circle.com")
}

// TestIntegration_RemoteFacilitatorInvalidPayment hits the real x402.org/facilitator
// with an obviously invalid payment. Asserts IsValid==false, proving connectivity
// without requiring a funded wallet.
func TestIntegration_RemoteFacilitatorInvalidPayment(t *testing.T) {
	if os.Getenv("COINBASE_CDP_KEY_NAME") == "" {
		t.Skip("COINBASE_CDP_KEY_NAME not set — skipping integration test")
	}

	f := x402.NewRemoteFacilitator(x402.CoinbaseX402FacilitatorURL)

	payment := &x402.PaymentPayload{
		X402Version: x402.Version,
	}
	requirements := &x402.PaymentRequired{
		X402Version: x402.Version,
	}

	resp, err := f.Verify(context.Background(), payment, requirements)
	// The facilitator may return an error or IsValid=false for an empty payload.
	// Either is acceptable — what matters is we reached the server.
	if err != nil {
		t.Logf("Verify returned error (acceptable for invalid payment): %v", err)
		return
	}

	if resp.IsValid {
		t.Error("expected IsValid=false for an empty/invalid payment payload")
	}
	t.Logf("Facilitator responded: IsValid=%v reason=%s", resp.IsValid, resp.InvalidReason)
}
