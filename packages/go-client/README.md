# AllSource Go Client

Go client library for the [AllSource](https://all-source.xyz) event store.

## Installation

```bash
go get github.com/all-source-os/allsource-go
```

## Quick Start

```go
package main

import (
	"context"
	"fmt"
	"log"

	allsource "github.com/all-source-os/allsource-go"
)

func main() {
	client, err := allsource.NewClient(allsource.Config{
		BaseURL: "https://allsource-query.fly.dev",
		APIKey:  "your-api-key",
	})
	if err != nil {
		log.Fatal(err)
	}

	ctx := context.Background()

	// Ingest an event
	event, err := client.IngestEvent(ctx, allsource.IngestEventInput{
		EventType: "user.signup",
		EntityID:  "user-123",
		Payload:   map[string]any{"email": "alice@example.com", "plan": "pro"},
		Metadata:  map[string]any{"source": "go-sdk"},
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Ingested event: %s\n", event.ID)

	// Query events
	resp, err := client.QueryEvents(ctx, &allsource.QueryEventsParams{
		EntityID: "user-123",
		Limit:    10,
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Found %d events\n", resp.Count)

	// Health check
	health, err := client.GetHealth(ctx)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Service status: %s\n", health.Status)
}
```

## API Reference

### `NewClient(cfg Config) (*Client, error)`

Creates a new AllSource client.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `BaseURL` | `string` | Yes | AllSource Query Service URL |
| `APIKey` | `string` | Yes | API key for authentication |
| `Timeout` | `time.Duration` | No | Request timeout (default 30s) |
| `HTTPClient` | `*http.Client` | No | Custom HTTP client |

### `IngestEvent(ctx, input) (*Event, error)`

Ingest a single event.

```go
event, err := client.IngestEvent(ctx, allsource.IngestEventInput{
    EventType: "order.placed",
    EntityID:  "order-456",
    Payload:   map[string]any{"total": 99.99, "currency": "USD"},
    Metadata:  map[string]any{"ip": "1.2.3.4"},
})
```

### `QueryEvents(ctx, params) (*QueryEventsResponse, error)`

Query events with optional filters. Pass `nil` for no filters.

```go
resp, err := client.QueryEvents(ctx, &allsource.QueryEventsParams{
    EntityID:  "user-123",
    EventType: "user.signup",
    Limit:     50,
    Offset:    0,
    StartTime: "2026-01-01T00:00:00Z",
    EndTime:   "2026-12-31T23:59:59Z",
})
for _, event := range resp.Events {
    fmt.Printf("%s: %s\n", event.Timestamp, event.EventType)
}
```

### `GetHealth(ctx) (*HealthResponse, error)`

Check service health.

```go
health, err := client.GetHealth(ctx)
fmt.Println(health.Status) // "ok"
```

## Error Handling

API errors are returned as `*allsource.Error` with status code and response body:

```go
event, err := client.IngestEvent(ctx, input)
if err != nil {
    var apiErr *allsource.Error
    if errors.As(err, &apiErr) {
        fmt.Printf("API error %d: %v\n", apiErr.Status, apiErr.Body)
    } else {
        fmt.Printf("Network error: %v\n", err)
    }
}
```

## Context Support

All methods accept `context.Context` for cancellation and deadline control:

```go
ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
defer cancel()

health, err := client.GetHealth(ctx)
```
