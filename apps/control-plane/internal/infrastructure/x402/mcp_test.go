package x402

import (
	"context"
	"encoding/json"
	"testing"
)

func mcpPricing() *PricingConfig {
	return &PricingConfig{
		Enabled:          true,
		RecipientAddress: "0xRecipient",
		Routes: map[string]RoutePricing{
			"MCP query_events": {
				Amount:   "1000",
				Networks: []string{NetworkBaseMainnet},
			},
		},
	}
}

func TestProcessMCPPayment_NoPayment_ReturnsRequirements(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, &mockChainSettler{txHash: "0xtx"})
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	meta := map[string]json.RawMessage{} // no x402/payment key

	resp, requirements, err := ProcessMCPPayment(context.Background(), f, mcpPricing(), logger, "query_events", meta, "tenant-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp != nil {
		t.Error("expected nil settle response")
	}
	if requirements == nil {
		t.Fatal("expected non-nil requirements")
		return
	}
	if len(requirements.Accepts) != 1 {
		t.Fatalf("accepts: want 1, got %d", len(requirements.Accepts))
	}
	if requirements.Accepts[0].Amount != "1000" {
		t.Errorf("amount: want 1000, got %s", requirements.Accepts[0].Amount)
	}

	// Should log payment.requested
	if len(mock.events) != 1 || mock.events[0].EventType != EventPaymentRequested {
		t.Error("expected payment.requested event")
	}
}

func TestProcessMCPPayment_WithPayment_Settles(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	f.RegisterEVM(&mockChainVerifier{payer: "0xPayer"}, &mockChainSettler{txHash: "0xmcp-tx"})
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	// Build payment payload
	auth := EIP3009Authorization{
		From: "0xPayer", To: "0xRecipient", Value: "1000",
		ValidAfter: "0", ValidBefore: "9999999999", Nonce: "0xmcp-nonce",
	}
	exactBytes, err := json.Marshal(ExactEIP3009Payload{Signature: "0xSig", Authorization: auth})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	payment := PaymentPayload{
		X402Version: Version,
		Payload:     exactBytes,
		Accepted:    PaymentRequirements{Scheme: SchemeExact, Network: NetworkBaseMainnet, Amount: "1000", PayTo: "0xRecipient"},
	}
	paymentBytes, err := json.Marshal(payment)
	if err != nil {
		t.Fatalf("marshal payment: %v", err)
	}

	meta := map[string]json.RawMessage{
		MCPPaymentMeta: paymentBytes,
	}

	resp, requirements, err := ProcessMCPPayment(context.Background(), f, mcpPricing(), logger, "query_events", meta, "tenant-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if requirements != nil {
		t.Error("expected nil requirements (payment provided)")
	}
	if resp == nil {
		t.Fatal("expected non-nil settle response")
		return
	}
	if !resp.Success {
		t.Errorf("expected success, got reason: %s", resp.ErrorReason)
	}
	if resp.Transaction != "0xmcp-tx" {
		t.Errorf("tx: want 0xmcp-tx, got %s", resp.Transaction)
	}

	// Should log payment.settled
	hasSettled := false
	for _, evt := range mock.events {
		if evt.EventType == EventPaymentSettled {
			hasSettled = true
			break
		}
	}
	if !hasSettled {
		t.Error("expected payment.settled event")
	}
}

func TestProcessMCPPayment_FreeTool_NilResponse(t *testing.T) {
	f := NewFacilitator(DefaultFacilitatorConfig())
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	// Tool not in pricing config
	meta := map[string]json.RawMessage{}
	resp, requirements, err := ProcessMCPPayment(context.Background(), f, mcpPricing(), logger, "free_tool", meta, "tenant-1")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if resp != nil || requirements != nil {
		t.Error("expected both nil for free tool")
	}
	if len(mock.events) != 0 {
		t.Error("expected no events for free tool")
	}
}
