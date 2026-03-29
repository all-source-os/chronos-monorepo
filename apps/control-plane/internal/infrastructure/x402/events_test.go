package x402

import (
	"context"
	"encoding/json"
	"testing"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// testCoreClient captures IngestEvent calls.
type testCoreClient struct {
	clients.CoreClient
	events []clients.IngestEventRequest
}

func (m *testCoreClient) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	m.events = append(m.events, req)
	return &clients.IngestEventResponse{EventID: "evt-test"}, nil
}

func TestEventLogger_LogSettled(t *testing.T) {
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	logger.LogSettled(context.Background(), "tenant-1", "0xdeadbeef", "0xPayer", NetworkBaseMainnet, "1000", "POST /api/v1/events")

	if len(mock.events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(mock.events))
	}

	evt := mock.events[0]
	if evt.EventType != EventPaymentSettled {
		t.Errorf("event_type: want %s, got %s", EventPaymentSettled, evt.EventType)
	}
	if evt.EntityID != "tenant-1" {
		t.Errorf("entity_id: want tenant-1, got %s", evt.EntityID)
	}
	if evt.Payload["transaction"] != "0xdeadbeef" {
		t.Errorf("tx: want 0xdeadbeef, got %v", evt.Payload["transaction"])
	}
	if evt.Payload["payer"] != "0xPayer" {
		t.Errorf("payer: want 0xPayer, got %v", evt.Payload["payer"])
	}
}

func TestEventLogger_LogRequested(t *testing.T) {
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	logger.LogRequested(context.Background(), "tenant-2", "GET /api/v1/events/query", NetworkBaseMainnet, "500")

	if len(mock.events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(mock.events))
	}
	if mock.events[0].EventType != EventPaymentRequested {
		t.Errorf("event_type: want %s, got %s", EventPaymentRequested, mock.events[0].EventType)
	}
}

func TestEventLogger_LogFailed(t *testing.T) {
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	logger.LogFailed(context.Background(), "tenant-3", "insufficient_balance", NetworkSolanaMainnet, "POST /api/v1/events")

	if len(mock.events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(mock.events))
	}
	evt := mock.events[0]
	if evt.EventType != EventPaymentFailed {
		t.Errorf("event_type: want %s, got %s", EventPaymentFailed, evt.EventType)
	}
	if evt.Payload["reason"] != "insufficient_balance" {
		t.Errorf("reason: want insufficient_balance, got %v", evt.Payload["reason"])
	}
}

func TestEventLogger_NilCoreClient(t *testing.T) {
	logger := NewEventLogger(nil)
	// Should not panic
	logger.LogSettled(context.Background(), "tenant-4", "0xtx", "0xPayer", NetworkBaseMainnet, "100", "/api/v1/events")
}

func TestEventLogger_AllEventTypes(t *testing.T) {
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)
	ctx := context.Background()

	logger.LogRequested(ctx, "t", "/ep", "net", "100")
	logger.LogVerified(ctx, "t", "payer", "net", "100")
	logger.LogSettled(ctx, "t", "0xtx", "payer", "net", "100", "/ep")
	logger.LogFailed(ctx, "t", "reason", "net", "/ep")

	if len(mock.events) != 4 {
		t.Fatalf("expected 4 events, got %d", len(mock.events))
	}

	expectedTypes := []string{EventPaymentRequested, EventPaymentVerified, EventPaymentSettled, EventPaymentFailed}
	for i, expected := range expectedTypes {
		if mock.events[i].EventType != expected {
			t.Errorf("event %d: want %s, got %s", i, expected, mock.events[i].EventType)
		}
	}
}

func TestEventLogger_PayloadIsSerializable(t *testing.T) {
	mock := &testCoreClient{}
	logger := NewEventLogger(mock)

	logger.LogSettled(context.Background(), "t", "0xtx", "0xPayer", NetworkBaseMainnet, "1000000", "/api/v1/events")

	// Verify the payload can be serialized to JSON (what Core expects)
	_, err := json.Marshal(mock.events[0])
	if err != nil {
		t.Fatalf("payload not serializable: %v", err)
	}
}
