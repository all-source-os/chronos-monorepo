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
	HeaderPayment         = "X-Payment"
	HeaderPaymentResponse = "X-Payment-Response"
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

// --- Server → Client (402 response body) ---

// PaymentRequired is the JSON body of an HTTP 402 response.
// It tells the client what payment is needed to access the resource.
type PaymentRequired struct {
	X402Version int                    `json:"x402Version"`
	Accepts     []PaymentRequirements  `json:"accepts"`
	Error       string                 `json:"error,omitempty"`
	Resource    *ResourceInfo          `json:"resource,omitempty"`
	Extensions  map[string]interface{} `json:"extensions,omitempty"`
}

// PaymentRequirements describes a single acceptable payment method.
type PaymentRequirements struct {
	Scheme            PaymentScheme          `json:"scheme"`
	Network           string                 `json:"network"`           // CAIP-2
	Asset             string                 `json:"asset"`             // Token contract address
	Amount            string                 `json:"amount"`            // Atomic units (USDC: 6 decimals)
	PayTo             string                 `json:"payTo"`             // Recipient wallet address
	MaxTimeoutSeconds int                    `json:"maxTimeoutSeconds"` // Min validity window
	Extra             map[string]interface{} `json:"extra,omitempty"`
}

// ResourceInfo describes the resource being paid for.
type ResourceInfo struct {
	URL         string `json:"url,omitempty"`
	Description string `json:"description,omitempty"`
	MimeType    string `json:"mimeType,omitempty"`
}

// --- Client → Server (X-Payment header, base64-encoded JSON) ---

// PaymentPayload is what the client sends in the X-Payment header.
type PaymentPayload struct {
	X402Version int                    `json:"x402Version"`
	Payload     json.RawMessage        `json:"payload"`  // Scheme-specific signed data
	Accepted    PaymentRequirements    `json:"accepted"` // Which requirement was accepted
	Resource    *ResourceInfo          `json:"resource,omitempty"`
	Extensions  map[string]interface{} `json:"extensions,omitempty"`
}

// --- EVM Exact scheme payloads ---

// ExactEIP3009Payload is the EVM exact-scheme payload using EIP-3009.
type ExactEIP3009Payload struct {
	Signature     string               `json:"signature"` // 65-byte hex EIP-712 sig
	Authorization EIP3009Authorization `json:"authorization"`
}

// EIP3009Authorization contains the EIP-3009 transferWithAuthorization fields.
type EIP3009Authorization struct {
	From        string `json:"from"`        // Payer address
	To          string `json:"to"`          // Recipient address (must match payTo)
	Value       string `json:"value"`       // Amount in atomic units
	ValidAfter  string `json:"validAfter"`  // Unix timestamp
	ValidBefore string `json:"validBefore"` // Unix timestamp
	Nonce       string `json:"nonce"`       // 32-byte random hex
}

// --- Solana Exact scheme payload ---

// ExactSVMPayload is the Solana exact-scheme payload (pre-signed transaction).
type ExactSVMPayload struct {
	Transaction string `json:"transaction"` // Base64-encoded Solana transaction
}

// --- Server → Client (X-Payment-Response header) ---

// SettlementResponse is returned in the X-Payment-Response header after settlement.
type SettlementResponse struct {
	Success     bool   `json:"success"`
	Transaction string `json:"transaction,omitempty"` // On-chain tx hash
	Network     string `json:"network,omitempty"`     // CAIP-2
	Payer       string `json:"payer,omitempty"`
	Amount      string `json:"amount,omitempty"`
	Error       string `json:"error,omitempty"`
}

// --- Facilitator API types ---

// FacilitatorRequest is the body for both /verify and /settle endpoints.
type FacilitatorRequest struct {
	X402Version         int             `json:"x402Version"`
	PaymentPayload      json.RawMessage `json:"paymentPayload"`
	PaymentRequirements json.RawMessage `json:"paymentRequirements"`
}

// VerifyResponse is the response from the facilitator's /verify endpoint.
type VerifyResponse struct {
	IsValid        bool   `json:"isValid"`
	InvalidReason  string `json:"invalidReason,omitempty"`
	InvalidMessage string `json:"invalidMessage,omitempty"`
	Payer          string `json:"payer,omitempty"`
}

// SettleResponse is the response from the facilitator's /settle endpoint.
type SettleResponse struct {
	Success      bool   `json:"success"`
	Transaction  string `json:"transaction,omitempty"`
	Network      string `json:"network,omitempty"`
	Payer        string `json:"payer,omitempty"`
	Amount       string `json:"amount,omitempty"`
	ErrorReason  string `json:"errorReason,omitempty"`
	ErrorMessage string `json:"errorMessage,omitempty"`
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
