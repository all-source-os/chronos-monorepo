package x402

import (
	"encoding/json"
	"testing"
)

func makeExactPayment(from, to, value, nonce, sig, network string) *PaymentPayload {
	auth := &TransferAuthorization{
		From:        from,
		To:          to,
		Value:       value,
		ValidAfter:  "0",
		ValidBefore: "9999999999",
		Nonce:       nonce,
	}
	exactBytes, _ := json.Marshal(ExactPaymentPayload{
		Signature:     sig,
		Authorization: auth,
	})
	return &PaymentPayload{
		X402Version: Version,
		Scheme:      SchemeExact,
		Network:     network,
		Payload:     exactBytes,
	}
}

func makeRequirements(amount, payTo, network string) *PaymentRequired {
	return &PaymentRequired{
		X402Version: Version,
		Accepts: []PaymentOffer{
			{
				Scheme:            SchemeExact,
				Network:           network,
				MaxAmountRequired: amount,
				PayTo:             payTo,
				Resource:          "/api/v1/events",
			},
		},
	}
}

func TestValidateExactPayment_HappyPath(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "1000", "0xnonce1", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	exact, offer, err := ValidateExactPayment(payment, requirements)
	if err != nil {
		t.Fatalf("ValidateExactPayment: %v", err)
	}
	if exact.Authorization.From != "0xSender" {
		t.Errorf("from: want 0xSender, got %s", exact.Authorization.From)
	}
	if offer.MaxAmountRequired != "1000" {
		t.Errorf("amount: want 1000, got %s", offer.MaxAmountRequired)
	}
}

func TestValidateExactPayment_OverpaymentAccepted(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "2000", "0xnonce2", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err != nil {
		t.Fatalf("overpayment should be accepted: %v", err)
	}
}

func TestValidateExactPayment_UnderpaymentRejected(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "500", "0xnonce3", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for underpayment")
	}
}

func TestValidateExactPayment_WrongRecipient(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xWrongAddr", "1000", "0xnonce4", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for wrong recipient")
	}
}

func TestValidateExactPayment_NoMatchingNetwork(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "1000", "0xnonce5", "0xSig", NetworkSolanaMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for network mismatch")
	}
}

func TestValidateExactPayment_WrongScheme(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "1000", "0xnonce6", "0xSig", NetworkBaseMainnet)
	payment.Scheme = "upto" // wrong scheme

	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for wrong scheme")
	}
}

func TestValidateExactPayment_MissingSignature(t *testing.T) {
	payment := makeExactPayment("0xSender", "0xRecipient", "1000", "0xnonce7", "", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for missing signature")
	}
}

func TestValidateExactPayment_MissingAuthorization(t *testing.T) {
	exactBytes, _ := json.Marshal(ExactPaymentPayload{Signature: "0xSig"})
	payment := &PaymentPayload{
		X402Version: Version,
		Scheme:      SchemeExact,
		Network:     NetworkBaseMainnet,
		Payload:     exactBytes,
	}
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for missing authorization")
	}
}
