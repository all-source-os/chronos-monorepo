package x402

import (
	"encoding/json"
	"testing"
)

func TestEncodeDecodeHeader(t *testing.T) {
	original := PaymentRequired{
		X402Version: Version,
		Accepts: []PaymentOffer{
			{
				Scheme:            SchemeExact,
				Network:           NetworkBaseMainnet,
				MaxAmountRequired: "1000",
				PayTo:             "0xRecipient",
				Resource:          "/api/v1/events",
				Description:       "Event ingestion",
			},
		},
	}

	encoded, err := EncodeHeader(original)
	if err != nil {
		t.Fatalf("EncodeHeader: %v", err)
	}
	if encoded == "" {
		t.Fatal("expected non-empty encoded string")
	}

	var decoded PaymentRequired
	if err := DecodeHeader(encoded, &decoded); err != nil {
		t.Fatalf("DecodeHeader: %v", err)
	}

	if decoded.X402Version != Version {
		t.Errorf("version: want %d, got %d", Version, decoded.X402Version)
	}
	if len(decoded.Accepts) != 1 {
		t.Fatalf("accepts: want 1 offer, got %d", len(decoded.Accepts))
	}
	offer := decoded.Accepts[0]
	if offer.Scheme != SchemeExact {
		t.Errorf("scheme: want %s, got %s", SchemeExact, offer.Scheme)
	}
	if offer.Network != NetworkBaseMainnet {
		t.Errorf("network: want %s, got %s", NetworkBaseMainnet, offer.Network)
	}
	if offer.MaxAmountRequired != "1000" {
		t.Errorf("amount: want 1000, got %s", offer.MaxAmountRequired)
	}
	if offer.PayTo != "0xRecipient" {
		t.Errorf("payTo: want 0xRecipient, got %s", offer.PayTo)
	}
}

func TestDecodeHeader_InvalidBase64(t *testing.T) {
	var v PaymentRequired
	err := DecodeHeader("not-valid-base64!!!", &v)
	if err == nil {
		t.Fatal("expected error for invalid base64")
	}
}

func TestDecodeHeader_InvalidJSON(t *testing.T) {
	// Valid base64 but not valid JSON
	var v PaymentRequired
	err := DecodeHeader("bm90LWpzb24=", &v)
	if err == nil {
		t.Fatal("expected error for invalid JSON")
	}
}

func TestPaymentPayloadRoundTrip(t *testing.T) {
	auth := &TransferAuthorization{
		From:        "0xSender",
		To:          "0xRecipient",
		Value:       "1000000",
		ValidAfter:  "0",
		ValidBefore: "9999999999",
		Nonce:       "0xabc123",
	}

	exactPayload, err := json.Marshal(ExactPaymentPayload{
		Signature:     "0xSig...",
		Authorization: auth,
	})
	if err != nil {
		t.Fatalf("marshal exact: %v", err)
	}

	payload := PaymentPayload{
		X402Version: Version,
		Scheme:      SchemeExact,
		Network:     NetworkBaseMainnet,
		Payload:     exactPayload,
	}

	// Encode to header
	encoded, err := EncodeHeader(payload)
	if err != nil {
		t.Fatalf("EncodeHeader: %v", err)
	}

	// Decode from header
	var decoded PaymentPayload
	if err := DecodeHeader(encoded, &decoded); err != nil {
		t.Fatalf("DecodeHeader: %v", err)
	}

	if decoded.X402Version != Version {
		t.Errorf("version: want %d, got %d", Version, decoded.X402Version)
	}
	if decoded.Scheme != SchemeExact {
		t.Errorf("scheme: want %s, got %s", SchemeExact, decoded.Scheme)
	}

	// Decode nested exact payload
	var exact ExactPaymentPayload
	if err := json.Unmarshal(decoded.Payload, &exact); err != nil {
		t.Fatalf("unmarshal exact: %v", err)
	}
	if exact.Authorization.From != "0xSender" {
		t.Errorf("from: want 0xSender, got %s", exact.Authorization.From)
	}
	if exact.Authorization.Nonce != "0xabc123" {
		t.Errorf("nonce: want 0xabc123, got %s", exact.Authorization.Nonce)
	}
}

func TestSettlementResponse_JSON(t *testing.T) {
	resp := SettlementResponse{
		Success:         true,
		TransactionHash: "0xdeadbeef",
		Network:         NetworkBaseMainnet,
	}

	data, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}

	var decoded SettlementResponse
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if !decoded.Success {
		t.Error("expected success=true")
	}
	if decoded.TransactionHash != "0xdeadbeef" {
		t.Errorf("tx: want 0xdeadbeef, got %s", decoded.TransactionHash)
	}
}
