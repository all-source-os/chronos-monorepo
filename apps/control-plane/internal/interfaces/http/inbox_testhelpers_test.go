package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"testing"
	"time"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/clients/emailprovider"
	"github.com/allsource/control-plane/internal/infrastructure/secrets"
)

// newSealer builds a deterministic (fixed-key) Sealer for tests, so a record
// sealed by one instance opens with any other.
func newSealer(t *testing.T) *secrets.Sealer {
	t.Helper()
	s, err := secrets.NewSealer(make([]byte, 32))
	if err != nil {
		t.Fatalf("NewSealer: %v", err)
	}
	return s
}

// fakeCore satisfies coreGateway for webhook-handler tests.
type fakeCore struct {
	tenant    string // GetConfig value; "" -> nil config (unknown connection)
	ingest    clients.IngestEventRequest
	ingested  bool
	ingestErr error
}

func (f *fakeCore) GetConfig(_ context.Context, key string) (*clients.ConfigEntryResponse, error) {
	if f.tenant == "" {
		return nil, nil
	}
	return &clients.ConfigEntryResponse{Key: key, Value: f.tenant}, nil
}

func (f *fakeCore) IngestEvent(_ context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error) {
	f.ingest = req
	f.ingested = true
	if f.ingestErr != nil {
		return nil, f.ingestErr
	}
	return &clients.IngestEventResponse{ID: "ev1"}, nil
}

// sampleMessage is a normalized inbound message for ingest assertions.
func sampleMessage() *emailprovider.Message {
	return &emailprovider.Message{
		ID: "msg1", ThreadID: "thr1", Subject: "Hi",
		From:    emailprovider.Address{Email: "a@b.com"},
		To:      []emailprovider.Address{{Email: "me@x.com"}},
		Snippet: "s", Body: "b", ReceivedAt: time.Unix(1750000000, 0).UTC(),
		Folder: "inbox", Labels: []string{"inbox"},
	}
}
