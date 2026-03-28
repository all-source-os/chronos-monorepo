package x402

import (
	"context"
	"encoding/json"
	"fmt"
)

// MCPPaymentMeta is the key used in JSON-RPC _meta for x402 payments.
const MCPPaymentMeta = "x402/payment"

// MCPPaymentRequest represents a payment embedded in an MCP tool call's _meta field.
type MCPPaymentRequest struct {
	Signature string          `json:"signature"`
	Payload   json.RawMessage `json:"payload"`
}

// MCPToolPricing describes x402 pricing metadata for an MCP tool.
type MCPToolPricing struct {
	Amount      string   `json:"amount"`
	Asset       string   `json:"asset"`
	Networks    []string `json:"networks"`
	Description string   `json:"description,omitempty"`
}

// ProcessMCPPayment handles x402 payment embedded in an MCP JSON-RPC call.
// If the tool requires payment and no payment is in _meta, returns requirements as an error.
// If payment is present, verifies and settles it.
func ProcessMCPPayment(
	ctx context.Context,
	facilitator *Facilitator,
	pricing *PricingConfig,
	logger *EventLogger,
	toolName string,
	meta map[string]json.RawMessage,
	tenantID string,
) (*SettleResponse, *PaymentRequired, error) {
	// Build route key from tool name
	routeKey := RouteKey("MCP", toolName)
	requirements := pricing.RequirementsForRoute(routeKey)
	if requirements == nil {
		// No pricing for this tool — free
		return nil, nil, nil
	}

	// Check for payment in _meta
	paymentRaw, ok := meta[MCPPaymentMeta]
	if !ok {
		// No payment — return requirements for the client
		if logger != nil && tenantID != "" {
			firstAccept := requirements.Accepts[0]
			logger.LogRequested(ctx, tenantID, routeKey, firstAccept.Network, firstAccept.Amount)
		}
		return nil, requirements, nil
	}

	// Decode the payment payload
	var payment PaymentPayload
	if err := json.Unmarshal(paymentRaw, &payment); err != nil {
		return nil, nil, fmt.Errorf("decode MCP payment: %w", err)
	}

	// Settle
	resp, err := facilitator.Settle(ctx, &payment, requirements)
	if err != nil {
		if logger != nil && tenantID != "" {
			logger.LogFailed(ctx, tenantID, err.Error(), payment.Accepted.Network, routeKey)
		}
		return nil, nil, fmt.Errorf("settle MCP payment: %w", err)
	}

	if !resp.Success {
		if logger != nil && tenantID != "" {
			logger.LogFailed(ctx, tenantID, resp.ErrorReason, payment.Accepted.Network, routeKey)
		}
		return resp, nil, nil
	}

	if logger != nil && tenantID != "" {
		logger.LogSettled(ctx, tenantID, resp.Transaction, resp.Payer, resp.Network, payment.Accepted.Amount, routeKey)
	}

	return resp, nil, nil
}
