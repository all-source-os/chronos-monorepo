# allsource-go

Go client for the [AllSource Event Store](https://all-source.xyz) API.

## Installation

```bash
go get github.com/all-source-os/allsource-go
```

## Quickstart

```go
package main

import (
	"context"
	"fmt"
	"log"

	allsource "github.com/all-source-os/allsource-go"
)

func main() {
	client := allsource.New("as_your_api_key", "https://api.all-source.xyz")

	ctx := context.Background()

	// Ingest an event
	event, err := client.Ingest(ctx, "user.signup", "user-123", map[string]any{
		"plan": "pro",
		"source": "web",
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Ingested event: %s\n", event.ID)

	// Query events
	result, err := client.Query(ctx, allsource.QueryOptions{
		EventType: "user.signup",
		Limit:     10,
	})
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Found %d events\n", result.Count)

	// Get projections
	projections, err := client.GetProjections(ctx)
	if err != nil {
		log.Fatal(err)
	}
	for _, p := range projections {
		fmt.Printf("Projection: %s (status: %s)\n", p.Name, p.Status)
	}

	// Get a single projection
	proj, err := client.GetProjection(ctx, "user-count")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Projection %s version %d\n", proj.Name, proj.Version)
}
```

## Error Handling

API errors are returned as `*allsource.APIError` with helper methods:

```go
_, err := client.Ingest(ctx, "user.signup", "user-123", data)
if err != nil {
	var apiErr *allsource.APIError
	if errors.As(err, &apiErr) {
		switch {
		case apiErr.IsUnauthorized():
			log.Fatal("Invalid API key")
		case apiErr.IsRateLimited():
			log.Println("Rate limited, retrying...")
		case apiErr.IsNotFound():
			log.Println("Resource not found")
		case apiErr.IsForbidden():
			log.Println("Insufficient permissions")
		default:
			log.Printf("API error %d: %s", apiErr.StatusCode, apiErr.Message)
		}
	}
}
```

## Query Options

```go
result, err := client.Query(ctx, allsource.QueryOptions{
	EventType: "order.placed",     // Filter by event type
	EntityID:  "order-456",        // Filter by entity ID
	Start:     "2026-01-01T00:00:00Z", // Start of time range (ISO-8601)
	End:       "2026-02-01T00:00:00Z", // End of time range (ISO-8601)
	Limit:     50,                 // Max results
	Offset:    10,                 // Skip first N results
})
```

## Custom HTTP Client

```go
httpClient := &http.Client{Timeout: 60 * time.Second}
client := allsource.NewWithHTTPClient("as_key", "https://api.all-source.xyz", httpClient)
```
