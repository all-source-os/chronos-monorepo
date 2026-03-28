package x402

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
	"time"
)

// mockChainVerifier implements ChainVerifier for testing.
type mockChainVerifier struct {
	payer string
	err   error
}

func (m *mockChainVerifier) Verify(_ context.Context, _ json.RawMessage, _ *PaymentRequirements) (string, error) {
	return m.payer, m.err
}

// mockChainSettler implements ChainSettler for testing.
type mockChainSettler struct {
	txHash string
	err    error
}

func (m *mockChainSettler) Settle(_ context.Context, _ json.RawMessage, _ *PaymentRequirements) (string, error) {
	return m.txHash, m.err
}

func testPayment(network, nonce string) (*PaymentPayload, *PaymentRequired) {
	auth := EIP3009Authorization{
		From: "0xPayer", To: "0xRecipient", Value: "1000",
		ValidAfter: "0", ValidBefore: "9999999999", Nonce: nonce,
	}
	exactBytes, err := json.Marshal(ExactEIP3009Payload{
		Signature: "0xSig", Authorization: auth,
	})
	if err != nil {
		panic(err)
	}

	payment := &PaymentPayload{
		X402Version: Version,
		Payload:     exactBytes,
		Accepted:    PaymentRequirements{Scheme: SchemeExact, Network: network, Amount: "1000", PayTo: "0xRecipient"},
	}
	requirements := &PaymentRequired{
		X402Version: Version,
		Accepts:     []PaymentRequirements{{Scheme: SchemeExact, Network: network, Amount: "1000", PayTo: "0xRecipient"}},
	}
	return payment, requirements
}

func TestFacilitator_Verify_HappyPath(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, nil)

	payment, requirements := testPayment(NetworkBaseMainnet, "0xnonce1")

	resp, err := f.Verify(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Verify: %v", err)
	}
	if !resp.IsValid {
		t.Errorf("expected valid, got reason: %s", resp.InvalidReason)
	}
	if resp.Payer != "0xPayer" {
		t.Errorf("payer: want 0xPayer, got %s", resp.Payer)
	}
}

func TestFacilitator_Verify_ChainVerificationFails(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{err: fmt.Errorf("insufficient balance")}, nil)

	payment, requirements := testPayment(NetworkBaseMainnet, "0xnonce2")

	resp, err := f.Verify(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Verify: %v", err)
	}
	if resp.IsValid {
		t.Error("expected invalid")
	}
	if resp.InvalidReason != "verification_failed" {
		t.Errorf("reason: want verification_failed, got %s", resp.InvalidReason)
	}
}

func TestFacilitator_Verify_UnsupportedNetwork(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	// No chains registered

	payment, requirements := testPayment(NetworkBaseMainnet, "0xnonce3")

	resp, err := f.Verify(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Verify: %v", err)
	}
	if resp.IsValid {
		t.Error("expected invalid for unsupported network")
	}
}

func TestFacilitator_Verify_NoMatchingRequirement(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, nil)

	payment, _ := testPayment(NetworkBaseMainnet, "0xnonce4")
	// Requirements for a different network
	requirements := &PaymentRequired{
		X402Version: Version,
		Accepts:     []PaymentRequirements{{Scheme: SchemeExact, Network: NetworkSolanaMainnet, Amount: "1000", PayTo: "0xRecipient"}},
	}

	resp, err := f.Verify(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Verify: %v", err)
	}
	if resp.IsValid {
		t.Error("expected invalid for no matching requirement")
	}
	if resp.InvalidReason != "no_matching_requirement" {
		t.Errorf("reason: want no_matching_requirement, got %s", resp.InvalidReason)
	}
}

func TestFacilitator_Settle_HappyPath(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(
		&mockChainVerifier{payer: "0xPayer"},
		&mockChainSettler{txHash: "0xdeadbeef"},
	)

	payment, requirements := testPayment(NetworkBaseMainnet, "0xsettle1")

	resp, err := f.Settle(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Settle: %v", err)
	}
	if !resp.Success {
		t.Errorf("expected success, got reason: %s", resp.ErrorReason)
	}
	if resp.Transaction != "0xdeadbeef" {
		t.Errorf("tx: want 0xdeadbeef, got %s", resp.Transaction)
	}
	if resp.Payer != "0xPayer" {
		t.Errorf("payer: want 0xPayer, got %s", resp.Payer)
	}
}

func TestFacilitator_Settle_SettlementFails(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(
		&mockChainVerifier{payer: "0xPayer"},
		&mockChainSettler{err: fmt.Errorf("gas estimation failed")},
	)

	payment, requirements := testPayment(NetworkBaseMainnet, "0xsettle2")

	resp, err := f.Settle(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Settle: %v", err)
	}
	if resp.Success {
		t.Error("expected failure")
	}
	if resp.ErrorReason != "settlement_failed" {
		t.Errorf("reason: want settlement_failed, got %s", resp.ErrorReason)
	}
}

func TestFacilitator_Settle_NonceReplayPrevention(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(
		&mockChainVerifier{payer: "0xPayer"},
		&mockChainSettler{txHash: "0xtx1"},
	)

	payment, requirements := testPayment(NetworkBaseMainnet, "0xsame-nonce")

	// First settlement should succeed
	resp1, err := f.Settle(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Settle 1: %v", err)
	}
	if !resp1.Success {
		t.Fatalf("first settle should succeed")
	}

	// Second settlement with same nonce should fail
	resp2, err := f.Settle(context.Background(), payment, requirements)
	if err != nil {
		t.Fatalf("Settle 2: %v", err)
	}
	if resp2.Success {
		t.Error("expected failure on replay")
	}
	if resp2.ErrorReason != "nonce_already_used" {
		t.Errorf("reason: want nonce_already_used, got %s", resp2.ErrorReason)
	}
}

func TestNonceTracker_Basic(t *testing.T) {
	nt := NewNonceTracker(1 * time.Hour)

	if !nt.Use("nonce1") {
		t.Error("first use should succeed")
	}
	if nt.Use("nonce1") {
		t.Error("second use should fail (replay)")
	}
	if !nt.IsUsed("nonce1") {
		t.Error("nonce1 should be marked used")
	}
	if nt.IsUsed("nonce2") {
		t.Error("nonce2 should not be used")
	}
	if nt.Size() != 1 {
		t.Errorf("size: want 1, got %d", nt.Size())
	}
}

func TestNonceTracker_Expiry(t *testing.T) {
	nt := NewNonceTracker(1 * time.Millisecond)

	nt.Use("expired-nonce")
	time.Sleep(5 * time.Millisecond)

	// After TTL, nonce should be cleaned up on next Use
	if !nt.Use("expired-nonce") {
		t.Error("expired nonce should be reusable")
	}
}

func TestChainPrefix(t *testing.T) {
	tests := []struct {
		network string
		want    string
	}{
		{"eip155:8453", "eip155"},
		{"solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp", "solana"},
		{"unknown", "unknown"},
	}

	for _, tt := range tests {
		if got := chainPrefix(tt.network); got != tt.want {
			t.Errorf("chainPrefix(%q) = %q, want %q", tt.network, got, tt.want)
		}
	}
}
