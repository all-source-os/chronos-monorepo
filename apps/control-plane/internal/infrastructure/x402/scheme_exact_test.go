package x402

import (
	"encoding/json"
	"testing"
)

func makeEVMPayment(from, to, value, nonce, sig, network string) *PaymentPayload {
	auth := EIP3009Authorization{
		From:        from,
		To:          to,
		Value:       value,
		ValidAfter:  "0",
		ValidBefore: "9999999999",
		Nonce:       nonce,
	}
	exactBytes, err := json.Marshal(ExactEIP3009Payload{
		Signature:     sig,
		Authorization: auth,
	})
	if err != nil {
		panic(err)
	}
	return &PaymentPayload{
		X402Version: Version,
		Payload:     exactBytes,
		Accepted: PaymentRequirements{
			Scheme:  SchemeExact,
			Network: network,
			Asset:   USDCBaseMainnet,
			Amount:  value,
			PayTo:   to,
		},
	}
}

func makeRequirements(amount, payTo, network string) *PaymentRequired {
	return &PaymentRequired{
		X402Version: Version,
		Accepts: []PaymentRequirements{
			{
				Scheme:  SchemeExact,
				Network: network,
				Asset:   USDCBaseMainnet,
				Amount:  amount,
				PayTo:   payTo,
			},
		},
	}
}

func TestValidateExactPayment_HappyPath(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "1000", "0xnonce1", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	exact, req, err := ValidateExactPayment(payment, requirements)
	if err != nil {
		t.Fatalf("ValidateExactPayment: %v", err)
	}
	if exact.Authorization.From != "0xSender" {
		t.Errorf("from: want 0xSender, got %s", exact.Authorization.From)
	}
	if req.Amount != "1000" {
		t.Errorf("amount: want 1000, got %s", req.Amount)
	}
}

func TestValidateExactPayment_OverpaymentAccepted(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "2000", "0xnonce2", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err != nil {
		t.Fatalf("overpayment should be accepted: %v", err)
	}
}

func TestValidateExactPayment_UnderpaymentRejected(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "500", "0xnonce3", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for underpayment")
	}
}

func TestValidateExactPayment_WrongRecipient(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xWrongAddr", "1000", "0xnonce4", "0xSig", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for wrong recipient")
	}
}

func TestValidateExactPayment_NoMatchingNetwork(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "1000", "0xnonce5", "0xSig", NetworkSolanaMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for network mismatch")
	}
}

func TestValidateExactPayment_WrongScheme(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "1000", "0xnonce6", "0xSig", NetworkBaseMainnet)
	payment.Accepted.Scheme = "upto" // wrong scheme

	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for wrong scheme")
	}
}

func TestValidateExactPayment_MissingSignature(t *testing.T) {
	payment := makeEVMPayment("0xSender", "0xRecipient", "1000", "0xnonce7", "", NetworkBaseMainnet)
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	_, _, err := ValidateExactPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for missing signature")
	}
}

func TestValidateExactPayment_MissingAuthorization(t *testing.T) {
	exactBytes, err := json.Marshal(ExactEIP3009Payload{Signature: "0xSig"})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	payment := &PaymentPayload{
		X402Version: Version,
		Payload:     exactBytes,
		Accepted: PaymentRequirements{
			Scheme:  SchemeExact,
			Network: NetworkBaseMainnet,
			Amount:  "1000",
			PayTo:   "0xRecipient",
		},
	}
	requirements := makeRequirements("1000", "0xRecipient", NetworkBaseMainnet)

	// Authorization fields are zero-valued but present — the recipient won't match
	_, _, valErr := ValidateExactPayment(payment, requirements)
	if valErr == nil {
		t.Fatal("expected error for zero-valued authorization")
	}
}

func TestValidateExactSVMPayment_HappyPath(t *testing.T) {
	txBytes, err := json.Marshal(ExactSVMPayload{Transaction: "base64EncodedTx..."})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	payment := &PaymentPayload{
		X402Version: Version,
		Payload:     txBytes,
		Accepted: PaymentRequirements{
			Scheme:  SchemeExact,
			Network: NetworkSolanaMainnet,
			Amount:  "1000",
			PayTo:   "SolRecipient",
		},
	}
	requirements := &PaymentRequired{
		X402Version: Version,
		Accepts: []PaymentRequirements{
			{Scheme: SchemeExact, Network: NetworkSolanaMainnet, Amount: "1000", PayTo: "SolRecipient"},
		},
	}

	svm, _, err := ValidateExactSVMPayment(payment, requirements)
	if err != nil {
		t.Fatalf("ValidateExactSVMPayment: %v", err)
	}
	if svm.Transaction != "base64EncodedTx..." {
		t.Errorf("tx: want base64EncodedTx..., got %s", svm.Transaction)
	}
}

func TestValidateExactSVMPayment_MissingTransaction(t *testing.T) {
	txBytes, err := json.Marshal(ExactSVMPayload{})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	payment := &PaymentPayload{
		X402Version: Version,
		Payload:     txBytes,
		Accepted: PaymentRequirements{
			Scheme:  SchemeExact,
			Network: NetworkSolanaMainnet,
			Amount:  "1000",
			PayTo:   "SolRecipient",
		},
	}
	requirements := &PaymentRequired{
		X402Version: Version,
		Accepts: []PaymentRequirements{
			{Scheme: SchemeExact, Network: NetworkSolanaMainnet, Amount: "1000", PayTo: "SolRecipient"},
		},
	}

	_, _, err = ValidateExactSVMPayment(payment, requirements)
	if err == nil {
		t.Fatal("expected error for missing transaction")
	}
}
