package http //nolint:revive // package name intentionally matches directory

import (
	"context"
	"encoding/json"

	"github.com/allsource/control-plane/internal/infrastructure/clients"
)

// coreGateway is the narrow slice of the Core client the email connector needs
// (webhook ingest + sealed-connection lookup). clients.CoreClient satisfies it.
type coreGateway interface {
	GetConfig(ctx context.Context, key string) (*clients.ConfigEntryResponse, error)
	IngestEvent(ctx context.Context, req clients.IngestEventRequest) (*clients.IngestEventResponse, error)
}

// grantConfigKey maps a connection id (the receiving address) to the Core config
// key holding its sealed record. "grant" is historical; a connection is a
// verified receiving address (grant_id ≡ the address).
func grantConfigKey(grantID string) string {
	return "connector:email:grant:" + grantID
}

// toMap round-trips a value through JSON into a generic map for a Core payload.
func toMap(v any) (map[string]any, error) {
	b, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		return nil, err
	}
	return m, nil
}
