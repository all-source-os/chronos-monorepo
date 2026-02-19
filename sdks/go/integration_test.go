package allsource

import (
	"context"
	"fmt"
	"os"
	"testing"
	"time"
)

// Integration tests require ALLSOURCE_TEST_CORE_URL to be set.
// Run: ALLSOURCE_TEST_CORE_URL=http://localhost:3280 ALLSOURCE_TEST_API_KEY=test-key go test -v -run Integration

func coreURL(t *testing.T) string {
	t.Helper()
	url := os.Getenv("ALLSOURCE_TEST_CORE_URL")
	if url == "" {
		t.Skip("ALLSOURCE_TEST_CORE_URL not set, skipping integration test")
	}
	return url
}

func apiKey() string {
	key := os.Getenv("ALLSOURCE_TEST_API_KEY")
	if key == "" {
		return "test-key"
	}
	return key
}

func uniqueEntity(prefix string) string {
	return fmt.Sprintf("%s-%d", prefix, time.Now().UnixNano())
}

func TestIntegrationHealth(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)

	result, err := client.Health(context.Background())
	if err != nil {
		t.Fatalf("health check failed: %v", err)
	}
	if result["status"] != "healthy" {
		t.Errorf("expected status healthy, got %v", result["status"])
	}
}

func TestIntegrationIngestAndQuery(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-single")

	// Ingest
	resp, err := client.Ingest(ctx, "go.test.created", entity, map[string]any{
		"test":  true,
		"value": 42,
	})
	if err != nil {
		t.Fatalf("ingest failed: %v", err)
	}
	if resp.EventID == "" {
		t.Error("event_id should not be empty")
	}
	if resp.Timestamp == "" {
		t.Error("timestamp should not be empty")
	}

	// Query back
	result, err := client.Query(ctx, QueryOptions{EntityID: entity})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if result.Count != 1 {
		t.Errorf("expected 1 event, got %d", result.Count)
	}
	if len(result.Events) != 1 {
		t.Fatalf("expected 1 event in array, got %d", len(result.Events))
	}
	if result.Events[0].EntityID != entity {
		t.Errorf("expected entity_id %s, got %s", entity, result.Events[0].EntityID)
	}
	if result.Events[0].EventType != "go.test.created" {
		t.Errorf("expected event_type go.test.created, got %s", result.Events[0].EventType)
	}
}

func TestIntegrationBatchIngest(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-batch")

	events := make([]map[string]any, 5)
	for i := 0; i < 5; i++ {
		events[i] = map[string]any{
			"event_type": fmt.Sprintf("go.test.batch.item%d", i),
			"entity_id":  entity,
			"payload":    map[string]any{"index": i},
		}
	}

	resp, err := client.IngestBatch(ctx, events)
	if err != nil {
		t.Fatalf("batch ingest failed: %v", err)
	}
	if resp.Total != 5 {
		t.Errorf("expected total 5, got %d", resp.Total)
	}
	if resp.Ingested != 5 {
		t.Errorf("expected ingested 5, got %d", resp.Ingested)
	}
	if len(resp.Events) != 5 {
		t.Errorf("expected 5 event responses, got %d", len(resp.Events))
	}

	// Query back
	result, err := client.Query(ctx, QueryOptions{EntityID: entity})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if result.Count != 5 {
		t.Errorf("expected 5 events, got %d", result.Count)
	}
}

func TestIntegrationQueryByType(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-type")
	eventType := fmt.Sprintf("go.test.typed.%d", time.Now().UnixNano())

	for i := 0; i < 3; i++ {
		_, err := client.Ingest(ctx, eventType, entity, map[string]any{"seq": i})
		if err != nil {
			t.Fatalf("ingest %d failed: %v", i, err)
		}
	}

	result, err := client.Query(ctx, QueryOptions{EventType: eventType})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if result.Count < 3 {
		t.Errorf("expected at least 3 events, got %d", result.Count)
	}
	for _, e := range result.Events {
		if e.EventType != eventType {
			t.Errorf("expected event_type %s, got %s", eventType, e.EventType)
		}
	}
}

func TestIntegrationQueryWithLimit(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-limit")

	for i := 0; i < 5; i++ {
		_, err := client.Ingest(ctx, "go.test.limit", entity, map[string]any{"seq": i})
		if err != nil {
			t.Fatalf("ingest failed: %v", err)
		}
	}

	result, err := client.Query(ctx, QueryOptions{EntityID: entity, Limit: 2})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if len(result.Events) != 2 {
		t.Errorf("expected 2 events with limit=2, got %d", len(result.Events))
	}
}

func TestIntegrationQueryNonexistent(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)

	result, err := client.Query(context.Background(), QueryOptions{
		EntityID: "nonexistent-entity-go-sdk-99999",
	})
	if err != nil {
		t.Fatalf("query should succeed with empty: %v", err)
	}
	if result.Count != 0 {
		t.Errorf("expected 0 events, got %d", result.Count)
	}
}

func TestIntegrationPayloadFidelity(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-fidelity")

	payload := map[string]any{
		"string":  "hello world",
		"number":  float64(42),
		"float":   3.14159,
		"boolean": true,
		"nested": map[string]any{
			"array": []any{float64(1), float64(2), float64(3)},
			"deep":  map[string]any{"key": "value"},
		},
	}

	_, err := client.Ingest(ctx, "go.test.fidelity", entity, payload)
	if err != nil {
		t.Fatalf("ingest failed: %v", err)
	}

	result, err := client.Query(ctx, QueryOptions{EntityID: entity})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if result.Count != 1 {
		t.Fatalf("expected 1 event, got %d", result.Count)
	}

	stored := result.Events[0].Payload
	if stored["string"] != "hello world" {
		t.Errorf("expected string 'hello world', got %v", stored["string"])
	}
	if stored["number"] != float64(42) {
		t.Errorf("expected number 42, got %v", stored["number"])
	}
	if stored["boolean"] != true {
		t.Errorf("expected boolean true, got %v", stored["boolean"])
	}
	nested, ok := stored["nested"].(map[string]any)
	if !ok {
		t.Fatal("expected nested object")
	}
	deep, ok := nested["deep"].(map[string]any)
	if !ok {
		t.Fatal("expected deep object")
	}
	if deep["key"] != "value" {
		t.Errorf("expected deep.key=value, got %v", deep["key"])
	}
}

func TestIntegrationEventOrdering(t *testing.T) {
	url := coreURL(t)
	client := New(apiKey(), url)
	ctx := context.Background()
	entity := uniqueEntity("go-sdk-order")

	for i := 0; i < 5; i++ {
		_, err := client.Ingest(ctx, "go.test.ordered", entity, map[string]any{"seq": float64(i)})
		if err != nil {
			t.Fatalf("ingest failed: %v", err)
		}
		time.Sleep(10 * time.Millisecond)
	}

	result, err := client.Query(ctx, QueryOptions{EntityID: entity})
	if err != nil {
		t.Fatalf("query failed: %v", err)
	}
	if result.Count != 5 {
		t.Fatalf("expected 5 events, got %d", result.Count)
	}

	for i, e := range result.Events {
		seq, ok := e.Payload["seq"].(float64)
		if !ok {
			t.Fatalf("expected seq as float64 at index %d", i)
		}
		if int(seq) != i {
			t.Errorf("expected seq=%d at index %d, got %v", i, i, seq)
		}
	}
}
