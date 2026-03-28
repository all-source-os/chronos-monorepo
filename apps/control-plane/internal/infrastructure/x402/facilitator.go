package x402

import (
	"context"
	"encoding/json"
	"fmt"
	"time"
)

// ChainVerifier verifies a payment signature on a specific chain.
// Implementations exist for EVM (EIP-3009) and SVM (SPL transfer).
type ChainVerifier interface {
	// Verify checks that the payment signature is valid and the payer has sufficient balance.
	// Returns the payer address or an error.
	Verify(ctx context.Context, payload json.RawMessage, requirements *PaymentRequirements) (payer string, err error)
}

// ChainSettler submits a payment transaction on-chain.
type ChainSettler interface {
	// Settle submits the payment on-chain and returns the transaction hash.
	Settle(ctx context.Context, payload json.RawMessage, requirements *PaymentRequirements) (txHash string, err error)
}

// FacilitatorConfig holds configuration for the facilitator.
type FacilitatorConfig struct {
	// SettlementTimeout is the max time to wait for on-chain settlement.
	SettlementTimeout time.Duration

	// NonceTTL is how long used nonces are tracked for replay protection.
	NonceTTL time.Duration
}

// DefaultFacilitatorConfig returns sensible defaults.
func DefaultFacilitatorConfig() FacilitatorConfig {
	return FacilitatorConfig{
		SettlementTimeout: 30 * time.Second,
		NonceTTL:          1 * time.Hour,
	}
}

// Facilitator verifies and settles x402 payments.
// It delegates chain-specific operations to injected ChainVerifier/ChainSettler implementations.
type Facilitator struct {
	verifiers map[string]ChainVerifier // network prefix → verifier ("eip155" or "solana")
	settlers  map[string]ChainSettler  // network prefix → settler
	nonces    *NonceTracker
	config    FacilitatorConfig
}

// NewFacilitator creates a Facilitator with the given chain implementations.
func NewFacilitator(config FacilitatorConfig) *Facilitator {
	return &Facilitator{
		verifiers: make(map[string]ChainVerifier),
		settlers:  make(map[string]ChainSettler),
		nonces:    NewNonceTracker(config.NonceTTL),
		config:    config,
	}
}

// RegisterEVM registers EVM chain verifier and settler.
func (f *Facilitator) RegisterEVM(verifier ChainVerifier, settler ChainSettler) {
	f.verifiers["eip155"] = verifier
	f.settlers["eip155"] = settler
}

// RegisterSVM registers Solana chain verifier and settler.
func (f *Facilitator) RegisterSVM(verifier ChainVerifier, settler ChainSettler) {
	f.verifiers["solana"] = verifier
	f.settlers["solana"] = settler
}

// Verify checks that a payment is valid without settling it.
func (f *Facilitator) Verify(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*VerifyResponse, error) {
	network := payment.Accepted.Network

	// Find the matching requirement
	var matched *PaymentRequirements
	for i := range requirements.Accepts {
		r := &requirements.Accepts[i]
		if r.Network == network && r.Scheme == payment.Accepted.Scheme {
			matched = r
			break
		}
	}
	if matched == nil {
		return &VerifyResponse{
			IsValid:       false,
			InvalidReason: "no_matching_requirement",
		}, nil
	}

	// Check nonce for replay
	nonce := f.extractNonce(payment)
	if nonce != "" && f.nonces.IsUsed(nonce) {
		return &VerifyResponse{
			IsValid:       false,
			InvalidReason: "nonce_already_used",
		}, nil
	}

	// Delegate to chain-specific verifier
	prefix := chainPrefix(network)
	verifier, ok := f.verifiers[prefix]
	if !ok {
		return &VerifyResponse{
			IsValid:       false,
			InvalidReason: fmt.Sprintf("unsupported_network_%s", prefix),
		}, nil
	}

	resp := &VerifyResponse{}
	payer, verifyErr := verifier.Verify(ctx, payment.Payload, matched)
	if verifyErr != nil {
		resp.InvalidReason = "verification_failed"
		resp.InvalidMessage = verifyErr.Error()
		return resp, nil //nolint:nilerr // verification failure is a valid response, not an error
	}

	resp.IsValid = true
	resp.Payer = payer
	return resp, nil
}

// Settle verifies and then settles a payment on-chain.
func (f *Facilitator) Settle(ctx context.Context, payment *PaymentPayload, requirements *PaymentRequired) (*SettleResponse, error) {
	// First verify
	verifyResp, err := f.Verify(ctx, payment, requirements)
	if err != nil {
		return nil, fmt.Errorf("verify: %w", err)
	}
	if !verifyResp.IsValid {
		return &SettleResponse{
			Success:      false,
			ErrorReason:  verifyResp.InvalidReason,
			ErrorMessage: verifyResp.InvalidMessage,
		}, nil
	}

	// Mark nonce as used (prevent double-settlement)
	nonce := f.extractNonce(payment)
	if nonce != "" {
		if !f.nonces.Use(nonce) {
			return &SettleResponse{
				Success:     false,
				ErrorReason: "nonce_already_used",
			}, nil
		}
	}

	// Find matching requirement
	network := payment.Accepted.Network
	var matched *PaymentRequirements
	for i := range requirements.Accepts {
		r := &requirements.Accepts[i]
		if r.Network == network && r.Scheme == payment.Accepted.Scheme {
			matched = r
			break
		}
	}

	// Delegate to chain-specific settler with timeout
	prefix := chainPrefix(network)
	settler, ok := f.settlers[prefix]
	if !ok {
		return &SettleResponse{
			Success:     false,
			ErrorReason: fmt.Sprintf("unsupported_network_%s", prefix),
		}, nil
	}

	settleCtx, cancel := context.WithTimeout(ctx, f.config.SettlementTimeout)
	defer cancel()

	txHash, err := settler.Settle(settleCtx, payment.Payload, matched)
	if err != nil {
		return &SettleResponse{
			Success:      false,
			ErrorReason:  "settlement_failed",
			ErrorMessage: err.Error(),
			Network:      network,
			Payer:        verifyResp.Payer,
		}, nil
	}

	return &SettleResponse{
		Success:     true,
		Transaction: txHash,
		Network:     network,
		Payer:       verifyResp.Payer,
	}, nil
}

// extractNonce pulls the nonce from an EVM payment payload. Returns "" for SVM.
func (f *Facilitator) extractNonce(payment *PaymentPayload) string {
	if IsEVMNetwork(payment.Accepted.Network) {
		var exact ExactEIP3009Payload
		if err := json.Unmarshal(payment.Payload, &exact); err == nil {
			return exact.Authorization.Nonce
		}
	}
	return ""
}

// chainPrefix extracts "eip155" or "solana" from a CAIP-2 network ID.
func chainPrefix(network string) string {
	for i, c := range network {
		if c == ':' {
			return network[:i]
		}
	}
	return network
}
