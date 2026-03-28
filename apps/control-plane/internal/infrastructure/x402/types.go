// Package x402 implements the x402 payment protocol (v2) for machine-to-machine payments.
// See https://github.com/coinbase/x402 for the specification.
package x402

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
)

// Version is the x402 protocol version supported by this implementation.
const Version = 2

// HTTP headers used by the x402 protocol.
const (
	HeaderPaymentRequired  = "X-Payment"
	HeaderPaymentSignature = "X-Payment-Signature"
	HeaderPaymentResponse  = "X-Payment-Response"
)

// CAIP-2 network identifiers.
const (
	NetworkBaseMainnet   = "eip155:8453"
	NetworkBaseSepolia   = "eip155:84532"
	NetworkSolanaMainnet = "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp"
	NetworkSolanaDevnet  = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1"
)

// USDC contract addresses.
const (
	USDCBaseMainnet   = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
	USDCBaseSepolia   = "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
	USDCSolanaMainnet = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
)

// PaymentScheme identifies the payment scheme type.
type PaymentScheme string

const (
	// SchemeExact requires an exact payment amount.
	SchemeExact PaymentScheme = "exact"
)

// PaymentRequired is the payload returned in the X-Payment header on HTTP 402.
// It tells the client what payment is needed to access the resource.
type PaymentRequired struct {
	// X402Version is the protocol version (must be 2).
	X402Version int `json:"x402Version"`

	// Accepts lists the payment options the server will accept.
	Accepts []PaymentOffer `json:"accepts"`

	// Error is an optional human-readable error message.
	Error string `json:"error,omitempty"`
}

// PaymentOffer describes a single acceptable payment method.
type PaymentOffer struct {
	// Scheme is the payment scheme (e.g., "exact").
	Scheme PaymentScheme `json:"scheme"`

	// Network is the CAIP-2 chain identifier (e.g., "eip155:8453").
	Network string `json:"network"`

	// MaxAmountRequired is the maximum payment amount in the token's smallest unit.
	// For USDC (6 decimals), "1000000" = $1.00.
	MaxAmountRequired string `json:"maxAmountRequired"`

	// Resource is the URL or resource identifier being paid for.
	Resource string `json:"resource"`

	// Description is a human-readable description of what is being paid for.
	Description string `json:"description,omitempty"`

	// MimeType is the MIME type of the resource.
	MimeType string `json:"mimeType,omitempty"`

	// PayTo is the recipient wallet address.
	PayTo string `json:"payTo"`

	// RequiredDeadlineSeconds is the minimum validity window for the payment authorization.
	RequiredDeadlineSeconds int64 `json:"requiredDeadlineSeconds,omitempty"`

	// OutputSchema describes the expected response format.
	OutputSchema *OutputSchema `json:"outputSchema,omitempty"`

	// Extra allows scheme-specific fields.
	Extra map[string]any `json:"extra,omitempty"`
}

// OutputSchema describes the expected response format for a paid resource.
type OutputSchema struct {
	MimeType    string `json:"mimeType,omitempty"`
	Description string `json:"description,omitempty"`
}

// PaymentPayload is what the client sends in the X-Payment-Signature header.
// It contains the signed payment authorization.
type PaymentPayload struct {
	// X402Version is the protocol version.
	X402Version int `json:"x402Version"`

	// Scheme is the payment scheme used.
	Scheme PaymentScheme `json:"scheme"`

	// Network is the CAIP-2 chain identifier.
	Network string `json:"network"`

	// Payload is the scheme-specific signed payment data.
	Payload json.RawMessage `json:"payload"`
}

// ExactPaymentPayload is the payload for the "exact" scheme.
type ExactPaymentPayload struct {
	// Signature is the EIP-3009 or SPL authorization signature.
	Signature string `json:"signature"`

	// Authorization contains the fields that were signed.
	Authorization *TransferAuthorization `json:"authorization"`
}

// TransferAuthorization contains the EIP-3009 transferWithAuthorization fields.
type TransferAuthorization struct {
	// From is the payer wallet address.
	From string `json:"from"`

	// To is the recipient wallet address.
	To string `json:"to"`

	// Value is the transfer amount in the token's smallest unit.
	Value string `json:"value"`

	// ValidAfter is the Unix timestamp after which the authorization is valid.
	ValidAfter string `json:"validAfter"`

	// ValidBefore is the Unix timestamp before which the authorization is valid.
	ValidBefore string `json:"validBefore"`

	// Nonce is a unique value to prevent replay attacks.
	Nonce string `json:"nonce"`
}

// SettlementResponse is the payload returned in the X-Payment-Response header.
type SettlementResponse struct {
	// Success indicates whether the payment was settled.
	Success bool `json:"success"`

	// TransactionHash is the on-chain transaction hash.
	TransactionHash string `json:"transaction,omitempty"`

	// Network is the CAIP-2 chain where settlement occurred.
	Network string `json:"network,omitempty"`

	// Error is set when settlement fails.
	Error string `json:"error,omitempty"`
}

// --- Facilitator API types ---

// VerifyRequest is the request to the facilitator's /verify endpoint.
type VerifyRequest struct {
	// Payment is the client's payment payload (base64-decoded).
	Payment *PaymentPayload `json:"payment"`

	// PaymentRequirements is the server's original payment requirements.
	PaymentRequirements *PaymentRequired `json:"paymentRequirements"`
}

// VerifyResponse is the response from the facilitator's /verify endpoint.
type VerifyResponse struct {
	// Valid indicates whether the payment signature is valid.
	Valid bool `json:"isValid"`

	// InvalidReason is set when Valid is false.
	InvalidReason string `json:"invalidReason,omitempty"`
}

// SettleRequest is the request to the facilitator's /settle endpoint.
type SettleRequest struct {
	// Payment is the verified payment payload.
	Payment *PaymentPayload `json:"payment"`

	// PaymentRequirements is the server's original payment requirements.
	PaymentRequirements *PaymentRequired `json:"paymentRequirements"`
}

// SettleResponse is the response from the facilitator's /settle endpoint.
type SettleResponse struct {
	// Success indicates whether settlement was completed.
	Success bool `json:"success"`

	// TransactionHash is the on-chain tx hash.
	TransactionHash string `json:"transaction,omitempty"`

	// Network is the CAIP-2 chain where settlement occurred.
	Network string `json:"network,omitempty"`

	// Error is set when settlement fails.
	Error string `json:"error,omitempty"`
}

// --- Encoding helpers ---

// EncodeHeader encodes a value as base64 JSON for use in HTTP headers.
func EncodeHeader(v any) (string, error) {
	data, err := json.Marshal(v)
	if err != nil {
		return "", fmt.Errorf("marshal header: %w", err)
	}
	return base64.StdEncoding.EncodeToString(data), nil
}

// DecodeHeader decodes a base64 JSON HTTP header value.
func DecodeHeader(encoded string, v any) error {
	data, err := base64.StdEncoding.DecodeString(encoded)
	if err != nil {
		return fmt.Errorf("base64 decode: %w", err)
	}
	if err := json.Unmarshal(data, v); err != nil {
		return fmt.Errorf("json unmarshal: %w", err)
	}
	return nil
}
