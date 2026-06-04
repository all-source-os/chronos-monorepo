package x402

import (
	"context"
	"log"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// Event type constants for x402 payment events written to Core.
const (
	EventPaymentRequested = "x402.payment.requested"
	EventPaymentVerified  = "x402.payment.verified"
	EventPaymentSettled   = "x402.payment.settled"
	EventPaymentFailed    = "x402.payment.failed"
	// EventAllowanceConsumed records an x402 priced-route call that was served
	// for free out of the tenant's included per-tier allowance (011). These are
	// reconciled into the tenant's x402_used counter so the allowance depletes
	// and overage (pay-as-you-go) kicks in once exhausted.
	EventAllowanceConsumed = "x402.allowance.consumed"
)

// EventLogger writes x402 payment events to Core for audit trail.
// All writes are non-blocking — errors are logged, not propagated.
type EventLogger struct {
	coreClient clients.CoreClient
}

// NewEventLogger creates an EventLogger. Accepts nil (logging is a no-op).
func NewEventLogger(coreClient clients.CoreClient) *EventLogger {
	return &EventLogger{coreClient: coreClient}
}

// LogRequested logs that an agent received a 402 response.
func (el *EventLogger) LogRequested(ctx context.Context, tenantID, endpoint, network, amount string) {
	el.log(ctx, EventPaymentRequested, tenantID, map[string]any{
		"endpoint": endpoint,
		"network":  network,
		"amount":   amount,
	})
}

// LogVerified logs that a payment signature was verified.
func (el *EventLogger) LogVerified(ctx context.Context, tenantID, payer, network, amount string) {
	el.log(ctx, EventPaymentVerified, tenantID, map[string]any{
		"payer":   payer,
		"network": network,
		"amount":  amount,
	})
}

// LogSettled logs that a payment was settled on-chain.
func (el *EventLogger) LogSettled(ctx context.Context, tenantID, txHash, payer, network, amount, endpoint string) {
	el.log(ctx, EventPaymentSettled, tenantID, map[string]any{
		"transaction": txHash,
		"payer":       payer,
		"network":     network,
		"amount":      amount,
		"endpoint":    endpoint,
	})
}

// LogAllowanceConsumed records that a priced route was served free from the
// tenant's included x402 allowance (no on-chain payment). One event per call;
// reconciled into x402_used.
func (el *EventLogger) LogAllowanceConsumed(ctx context.Context, tenantID, endpoint string) {
	el.log(ctx, EventAllowanceConsumed, tenantID, map[string]any{
		"endpoint": endpoint,
	})
}

// LogFailed logs a payment verification or settlement failure.
func (el *EventLogger) LogFailed(ctx context.Context, tenantID, reason, network, endpoint string) {
	el.log(ctx, EventPaymentFailed, tenantID, map[string]any{
		"reason":   reason,
		"network":  network,
		"endpoint": endpoint,
	})
}

func (el *EventLogger) log(ctx context.Context, eventType, tenantID string, payload map[string]any) {
	if el.coreClient == nil {
		return
	}

	_, err := el.coreClient.IngestEvent(ctx, clients.IngestEventRequest{
		EventType: eventType,
		EntityID:  tenantID,
		Payload:   payload,
	})
	if err != nil {
		log.Printf("[x402] failed to write %s event for %s: %v", eventType, tenantID, err)
	}
}
