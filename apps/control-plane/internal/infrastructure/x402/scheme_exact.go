package x402

import (
	"encoding/json"
	"fmt"
	"math/big"
)

// ValidateExactPayment checks that an exact-scheme payment meets the requirements.
// Returns the decoded authorization or an error describing what's wrong.
func ValidateExactPayment(payment *PaymentPayload, requirements *PaymentRequired) (*ExactPaymentPayload, *PaymentOffer, error) {
	if payment.Scheme != SchemeExact {
		return nil, nil, fmt.Errorf("expected scheme %q, got %q", SchemeExact, payment.Scheme)
	}

	// Find a matching offer for this network
	var offer *PaymentOffer
	for i := range requirements.Accepts {
		if requirements.Accepts[i].Network == payment.Network && requirements.Accepts[i].Scheme == SchemeExact {
			offer = &requirements.Accepts[i]
			break
		}
	}
	if offer == nil {
		return nil, nil, fmt.Errorf("no matching offer for network %q scheme %q", payment.Network, payment.Scheme)
	}

	// Decode the scheme-specific payload
	var exact ExactPaymentPayload
	if err := json.Unmarshal(payment.Payload, &exact); err != nil {
		return nil, nil, fmt.Errorf("decode exact payload: %w", err)
	}

	if exact.Authorization == nil {
		return nil, nil, fmt.Errorf("missing authorization in exact payment")
	}

	if exact.Signature == "" {
		return nil, nil, fmt.Errorf("missing signature in exact payment")
	}

	// Verify the amount meets or exceeds the required amount
	required, ok := new(big.Int).SetString(offer.MaxAmountRequired, 10)
	if !ok {
		return nil, nil, fmt.Errorf("invalid required amount: %s", offer.MaxAmountRequired)
	}
	actual, ok := new(big.Int).SetString(exact.Authorization.Value, 10)
	if !ok {
		return nil, nil, fmt.Errorf("invalid payment value: %s", exact.Authorization.Value)
	}
	if actual.Cmp(required) < 0 {
		return nil, nil, fmt.Errorf("payment amount %s less than required %s", exact.Authorization.Value, offer.MaxAmountRequired)
	}

	// Verify recipient matches
	if exact.Authorization.To != offer.PayTo {
		return nil, nil, fmt.Errorf("payment recipient %s does not match required %s", exact.Authorization.To, offer.PayTo)
	}

	return &exact, offer, nil
}
