package x402

import (
	"encoding/json"
	"fmt"
	"math/big"
)

// ValidateExactPayment checks that an exact-scheme payment meets the requirements.
// Returns the decoded EIP-3009 authorization and the matched requirements, or an error.
// For Solana payments, use ValidateExactSVMPayment instead.
func ValidateExactPayment(payment *PaymentPayload, requirements *PaymentRequired) (*ExactEIP3009Payload, *PaymentRequirements, error) {
	accepted := payment.Accepted

	if accepted.Scheme != SchemeExact {
		return nil, nil, fmt.Errorf("expected scheme %q, got %q", SchemeExact, accepted.Scheme)
	}

	// Find a matching requirement for the accepted network + scheme
	var matched *PaymentRequirements
	for i := range requirements.Accepts {
		r := &requirements.Accepts[i]
		if r.Network == accepted.Network && r.Scheme == SchemeExact {
			matched = r
			break
		}
	}
	if matched == nil {
		return nil, nil, fmt.Errorf("no matching requirement for network %q scheme %q", accepted.Network, accepted.Scheme)
	}

	// Decode the EIP-3009 payload
	var exact ExactEIP3009Payload
	if err := json.Unmarshal(payment.Payload, &exact); err != nil {
		return nil, nil, fmt.Errorf("decode exact payload: %w", err)
	}

	if exact.Signature == "" {
		return nil, nil, fmt.Errorf("missing signature in exact payment")
	}

	// Verify the amount meets or exceeds the required amount
	required, ok := new(big.Int).SetString(matched.Amount, 10)
	if !ok {
		return nil, nil, fmt.Errorf("invalid required amount: %s", matched.Amount)
	}
	actual, ok := new(big.Int).SetString(exact.Authorization.Value, 10)
	if !ok {
		return nil, nil, fmt.Errorf("invalid payment value: %s", exact.Authorization.Value)
	}
	if actual.Cmp(required) < 0 {
		return nil, nil, fmt.Errorf("payment amount %s less than required %s", exact.Authorization.Value, matched.Amount)
	}

	// Verify recipient matches
	if exact.Authorization.To != matched.PayTo {
		return nil, nil, fmt.Errorf("payment recipient %s does not match required %s", exact.Authorization.To, matched.PayTo)
	}

	return &exact, matched, nil
}

// ValidateExactSVMPayment checks that a Solana exact-scheme payment is present.
// Returns the decoded transaction payload and matched requirements.
// Actual transaction verification happens in the facilitator (on-chain simulation).
func ValidateExactSVMPayment(payment *PaymentPayload, requirements *PaymentRequired) (*ExactSVMPayload, *PaymentRequirements, error) {
	accepted := payment.Accepted

	if accepted.Scheme != SchemeExact {
		return nil, nil, fmt.Errorf("expected scheme %q, got %q", SchemeExact, accepted.Scheme)
	}

	var matched *PaymentRequirements
	for i := range requirements.Accepts {
		r := &requirements.Accepts[i]
		if r.Network == accepted.Network && r.Scheme == SchemeExact {
			matched = r
			break
		}
	}
	if matched == nil {
		return nil, nil, fmt.Errorf("no matching requirement for network %q", accepted.Network)
	}

	var svm ExactSVMPayload
	if err := json.Unmarshal(payment.Payload, &svm); err != nil {
		return nil, nil, fmt.Errorf("decode SVM payload: %w", err)
	}

	if svm.Transaction == "" {
		return nil, nil, fmt.Errorf("missing transaction in SVM payment")
	}

	return &svm, matched, nil
}
