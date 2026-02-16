package allsource

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestNewClient_RequiresBaseURL(t *testing.T) {
	_, err := NewClient(Config{APIKey: "test-key"})
	if err == nil {
		t.Fatal("expected error for missing BaseURL")
	}
}

func TestNewClient_RequiresAPIKey(t *testing.T) {
	_, err := NewClient(Config{BaseURL: "http://localhost"})
	if err == nil {
		t.Fatal("expected error for missing APIKey")
	}
}

func TestNewClient_TrimsTrailingSlash(t *testing.T) {
	c, err := NewClient(Config{BaseURL: "http://localhost:3902/", APIKey: "key"})
	if err != nil {
		t.Fatal(err)
	}
	if c.baseURL != "http://localhost:3902" {
		t.Fatalf("expected trailing slash trimmed, got %q", c.baseURL)
	}
}

func TestGetHealth(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/health" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		if r.Header.Get("X-API-Key") != "test-key" {
			t.Errorf("missing API key header")
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{"status": "ok", "version": "1.0"})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "test-key"})
	resp, err := c.GetHealth(context.Background())
	if err != nil {
		t.Fatal(err)
	}
	if resp.Status != "ok" {
		t.Fatalf("expected status ok, got %q", resp.Status)
	}
	if resp.Extra["version"] != "1.0" {
		t.Fatalf("expected extra version 1.0, got %v", resp.Extra["version"])
	}
}

func TestIngestEvent(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/api/events" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		if r.Header.Get("Content-Type") != "application/json" {
			t.Errorf("missing content-type")
		}
		if r.Header.Get("X-API-Key") != "my-key" {
			t.Errorf("missing API key header")
		}

		var body IngestEventInput
		json.NewDecoder(r.Body).Decode(&body)
		if body.EventType != "user.signup" {
			t.Errorf("expected event_type user.signup, got %q", body.EventType)
		}
		if body.EntityID != "user-1" {
			t.Errorf("expected entity_id user-1, got %q", body.EntityID)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(Event{
			ID:        "evt-123",
			EventType: body.EventType,
			EntityID:  body.EntityID,
			Payload:   body.Payload,
			Metadata:  map[string]any{},
			Timestamp: "2026-02-16T00:00:00Z",
		})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "my-key"})
	event, err := c.IngestEvent(context.Background(), IngestEventInput{
		EventType: "user.signup",
		EntityID:  "user-1",
		Payload:   map[string]any{"email": "test@example.com"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if event.ID != "evt-123" {
		t.Fatalf("expected ID evt-123, got %q", event.ID)
	}
	if event.EventType != "user.signup" {
		t.Fatalf("expected event_type user.signup, got %q", event.EventType)
	}
}

func TestIngestEvent_WithMetadata(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var body map[string]any
		json.NewDecoder(r.Body).Decode(&body)
		if body["metadata"] == nil {
			t.Error("expected metadata in body")
		}
		meta := body["metadata"].(map[string]any)
		if meta["source"] != "go-sdk" {
			t.Errorf("expected source go-sdk, got %v", meta["source"])
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(Event{
			ID:        "evt-456",
			EventType: "order.placed",
			EntityID:  "order-1",
			Payload:   map[string]any{},
			Metadata:  map[string]any{"source": "go-sdk"},
			Timestamp: "2026-02-16T00:00:00Z",
		})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "key"})
	event, err := c.IngestEvent(context.Background(), IngestEventInput{
		EventType: "order.placed",
		EntityID:  "order-1",
		Payload:   map[string]any{"total": 99.99},
		Metadata:  map[string]any{"source": "go-sdk"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if event.Metadata["source"] != "go-sdk" {
		t.Fatalf("expected metadata source go-sdk, got %v", event.Metadata["source"])
	}
}

func TestQueryEvents(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/api/events" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(QueryEventsResponse{
			Events: []Event{{ID: "evt-1", EventType: "user.signup", EntityID: "u-1", Payload: map[string]any{}, Metadata: map[string]any{}, Timestamp: "2026-02-16T00:00:00Z"}},
			Count:  1,
		})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "key"})
	resp, err := c.QueryEvents(context.Background(), nil)
	if err != nil {
		t.Fatal(err)
	}
	if resp.Count != 1 {
		t.Fatalf("expected count 1, got %d", resp.Count)
	}
	if len(resp.Events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(resp.Events))
	}
}

func TestQueryEvents_WithParams(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		if q.Get("entity_id") != "user-1" {
			t.Errorf("expected entity_id=user-1, got %q", q.Get("entity_id"))
		}
		if q.Get("event_type") != "user.signup" {
			t.Errorf("expected event_type=user.signup, got %q", q.Get("event_type"))
		}
		if q.Get("limit") != "10" {
			t.Errorf("expected limit=10, got %q", q.Get("limit"))
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(QueryEventsResponse{Events: []Event{}, Count: 0})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "key"})
	_, err := c.QueryEvents(context.Background(), &QueryEventsParams{
		EntityID:  "user-1",
		EventType: "user.signup",
		Limit:     10,
	})
	if err != nil {
		t.Fatal(err)
	}
}

func TestAPIError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusUnauthorized)
		json.NewEncoder(w).Encode(map[string]string{"error": "invalid api key"})
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "bad-key"})
	_, err := c.GetHealth(context.Background())
	if err == nil {
		t.Fatal("expected error for 401")
	}
	apiErr, ok := err.(*Error)
	if !ok {
		t.Fatalf("expected *Error, got %T", err)
	}
	if apiErr.Status != 401 {
		t.Fatalf("expected status 401, got %d", apiErr.Status)
	}
}

func TestAPIError_TextBody(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
		w.Write([]byte("internal server error"))
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "key"})
	_, err := c.GetHealth(context.Background())
	if err == nil {
		t.Fatal("expected error for 500")
	}
	apiErr, ok := err.(*Error)
	if !ok {
		t.Fatalf("expected *Error, got %T", err)
	}
	if apiErr.Status != 500 {
		t.Fatalf("expected status 500, got %d", apiErr.Status)
	}
	if apiErr.Body != "internal server error" {
		t.Fatalf("expected text body, got %v", apiErr.Body)
	}
}

func TestContextCancellation(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// This handler will be slow, but context cancellation should abort first
		select {
		case <-r.Context().Done():
			return
		}
	}))
	defer srv.Close()

	c, _ := NewClient(Config{BaseURL: srv.URL, APIKey: "key"})
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // cancel immediately

	_, err := c.GetHealth(ctx)
	if err == nil {
		t.Fatal("expected error for cancelled context")
	}
}
