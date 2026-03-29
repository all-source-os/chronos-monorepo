package usecases

import (
	"context"
	"fmt"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// AgentPaymentHistoryRequest represents a request to query payment history.
type AgentPaymentHistoryRequest struct {
	TenantID string
	Since    string // RFC3339 timestamp
	Until    string // RFC3339 timestamp
	Network  string // optional filter: "evm", "solana", or "" for all
	Limit    int
	Offset   int
}

// AgentPaymentEntry represents a single payment event from the history.
type AgentPaymentEntry struct {
	EventType   string         `json:"event_type"`
	Timestamp   string         `json:"timestamp"`
	Amount      string         `json:"amount,omitempty"`
	Network     string         `json:"network,omitempty"`
	Transaction string         `json:"transaction,omitempty"`
	Payer       string         `json:"payer,omitempty"`
	Endpoint    string         `json:"endpoint,omitempty"`
	Status      string         `json:"status,omitempty"`
	Payload     map[string]any `json:"payload,omitempty"`
}

// AgentPaymentHistoryResponse wraps the payment history.
type AgentPaymentHistoryResponse struct {
	Payments []AgentPaymentEntry `json:"payments"`
	Total    int                 `json:"total"`
}

// GetAgentPaymentHistoryUseCase queries Core for x402 payment events.
type GetAgentPaymentHistoryUseCase struct {
	coreClient clients.CoreClient
}

// NewGetAgentPaymentHistoryUseCase creates a new GetAgentPaymentHistoryUseCase.
func NewGetAgentPaymentHistoryUseCase(coreClient clients.CoreClient) *GetAgentPaymentHistoryUseCase {
	return &GetAgentPaymentHistoryUseCase{coreClient: coreClient}
}

// Execute queries payment history from Core events.
func (uc *GetAgentPaymentHistoryUseCase) Execute(ctx context.Context, req AgentPaymentHistoryRequest) (*AgentPaymentHistoryResponse, error) {
	if uc.coreClient == nil {
		return &AgentPaymentHistoryResponse{Payments: []AgentPaymentEntry{}}, nil
	}

	// Query Core for x402 payment events for this tenant
	queryReq := clients.QueryEventsRequest{
		EntityID:  req.TenantID,
		EventType: "x402.payment.", // prefix match
		Limit:     req.Limit,
		Offset:    req.Offset,
	}

	if req.Since != "" {
		queryReq.Since = req.Since
	}
	if req.Until != "" {
		queryReq.Until = req.Until
	}

	resp, err := uc.coreClient.QueryEvents(ctx, queryReq)
	if err != nil {
		return nil, fmt.Errorf("query payment events: %w", err)
	}

	payments := make([]AgentPaymentEntry, 0, len(resp.Events))
	for _, evt := range resp.Events {
		entry := AgentPaymentEntry{
			EventType: evt.EventType,
			Timestamp: evt.Timestamp,
			Payload:   evt.Payload,
		}

		// Extract commonly queried fields from payload
		if v, ok := evt.Payload["amount"].(string); ok {
			entry.Amount = v
		}
		if v, ok := evt.Payload["network"].(string); ok {
			entry.Network = v
		}
		if v, ok := evt.Payload["transaction"].(string); ok {
			entry.Transaction = v
		}
		if v, ok := evt.Payload["payer"].(string); ok {
			entry.Payer = v
		}
		if v, ok := evt.Payload["endpoint"].(string); ok {
			entry.Endpoint = v
		}

		// Apply network filter if specified
		if req.Network != "" {
			if req.Network == "evm" && !isEVMNetworkPayload(entry.Network) {
				continue
			}
			if req.Network == "solana" && !isSVMNetworkPayload(entry.Network) {
				continue
			}
		}

		payments = append(payments, entry)
	}

	return &AgentPaymentHistoryResponse{
		Payments: payments,
		Total:    resp.Count,
	}, nil
}

func isEVMNetworkPayload(network string) bool {
	return len(network) > 6 && network[:6] == "eip155"
}

func isSVMNetworkPayload(network string) bool {
	return len(network) > 6 && network[:6] == "solana"
}
