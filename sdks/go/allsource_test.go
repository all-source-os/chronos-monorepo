package allsource

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"
)

func setupTestServer(handler http.HandlerFunc) (*httptest.Server, *Client) {
	srv := httptest.NewServer(handler)
	client := New("test-api-key", srv.URL)
	return srv, client
}

func TestNew(t *testing.T) {
	c := New("my-key", "https://api.example.com")
	if c.apiKey != "my-key" {
		t.Errorf("expected apiKey my-key, got %s", c.apiKey)
	}
	if c.baseURL != "https://api.example.com" {
		t.Errorf("expected baseURL https://api.example.com, got %s", c.baseURL)
	}
}

func TestIngest(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/api/events" {
			t.Errorf("expected /api/events, got %s", r.URL.Path)
		}
		if r.Header.Get("Authorization") != "Bearer test-api-key" {
			t.Errorf("expected Bearer test-api-key, got %s", r.Header.Get("Authorization"))
		}

		var body map[string]any
		json.NewDecoder(r.Body).Decode(&body)
		if body["event_type"] != "user.signup" {
			t.Errorf("expected event_type user.signup, got %v", body["event_type"])
		}
		if body["entity_id"] != "user-123" {
			t.Errorf("expected entity_id user-123, got %v", body["entity_id"])
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"data": map[string]any{
				"id":         "evt-1",
				"entity_id":  "user-123",
				"event_type": "user.signup",
				"payload":    map[string]any{"plan": "pro"},
				"timestamp":  "2026-01-01T00:00:00Z",
				"version":    1,
			},
		})
	})
	defer srv.Close()

	event, err := client.Ingest(context.Background(), "user.signup", "user-123", map[string]any{"plan": "pro"})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if event.ID != "evt-1" {
		t.Errorf("expected ID evt-1, got %s", event.ID)
	}
	if event.EventType != "user.signup" {
		t.Errorf("expected EventType user.signup, got %s", event.EventType)
	}
}

func TestQuery(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/api/events" {
			t.Errorf("expected /api/events, got %s", r.URL.Path)
		}
		if r.URL.Query().Get("event_type") != "user.signup" {
			t.Errorf("expected event_type=user.signup, got %s", r.URL.Query().Get("event_type"))
		}
		if r.URL.Query().Get("limit") != "10" {
			t.Errorf("expected limit=10, got %s", r.URL.Query().Get("limit"))
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"data": map[string]any{
				"events": []map[string]any{
					{
						"id":         "evt-1",
						"entity_id":  "user-123",
						"event_type": "user.signup",
						"payload":    map[string]any{},
						"timestamp":  "2026-01-01T00:00:00Z",
						"version":    1,
					},
				},
				"count": 1,
			},
		})
	})
	defer srv.Close()

	result, err := client.Query(context.Background(), QueryOptions{
		EventType: "user.signup",
		Limit:     10,
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Count != 1 {
		t.Errorf("expected count 1, got %d", result.Count)
	}
	if len(result.Events) != 1 {
		t.Errorf("expected 1 event, got %d", len(result.Events))
	}
	if result.Events[0].ID != "evt-1" {
		t.Errorf("expected event ID evt-1, got %s", result.Events[0].ID)
	}
}

func TestGetProjections(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/projections" {
			t.Errorf("expected /api/projections, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"data": []map[string]any{
				{
					"id":            "proj-1",
					"name":          "user-count",
					"version":       1,
					"status":        "running",
					"initial_state": map[string]any{},
					"definition":    "count(*)",
					"created_at":    "2026-01-01T00:00:00Z",
					"updated_at":    "2026-01-01T00:00:00Z",
				},
			},
		})
	})
	defer srv.Close()

	projections, err := client.GetProjections(context.Background())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(projections) != 1 {
		t.Fatalf("expected 1 projection, got %d", len(projections))
	}
	if projections[0].Name != "user-count" {
		t.Errorf("expected name user-count, got %s", projections[0].Name)
	}
}

func TestGetProjection(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/projections/user-count" {
			t.Errorf("expected /api/projections/user-count, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"data": map[string]any{
				"id":            "proj-1",
				"name":          "user-count",
				"version":       1,
				"status":        "running",
				"initial_state": map[string]any{},
				"definition":    "count(*)",
				"created_at":    "2026-01-01T00:00:00Z",
				"updated_at":    "2026-01-01T00:00:00Z",
			},
		})
	})
	defer srv.Close()

	p, err := client.GetProjection(context.Background(), "user-count")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if p.Name != "user-count" {
		t.Errorf("expected name user-count, got %s", p.Name)
	}
	if p.Status != "running" {
		t.Errorf("expected status running, got %s", p.Status)
	}
}

func TestHealth(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/health" {
			t.Errorf("expected /health, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"status": "healthy",
		})
	})
	defer srv.Close()

	result, err := client.Health(context.Background())
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result["status"] != "healthy" {
		t.Errorf("expected status healthy, got %v", result["status"])
	}
}

func TestAPIError401(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(401)
		json.NewEncoder(w).Encode(map[string]any{
			"error": map[string]any{
				"code":    "unauthorized",
				"message": "invalid api key",
			},
		})
	})
	defer srv.Close()

	_, err := client.Health(context.Background())
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T", err)
	}
	if !apiErr.IsUnauthorized() {
		t.Errorf("expected IsUnauthorized, got status %d", apiErr.StatusCode)
	}
	if apiErr.Code != "unauthorized" {
		t.Errorf("expected code unauthorized, got %s", apiErr.Code)
	}
}

func TestAPIError403(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(403)
		json.NewEncoder(w).Encode(map[string]any{
			"error": map[string]any{
				"code":    "forbidden",
				"message": "insufficient permissions",
			},
		})
	})
	defer srv.Close()

	_, err := client.Health(context.Background())
	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T", err)
	}
	if !apiErr.IsForbidden() {
		t.Errorf("expected IsForbidden, got status %d", apiErr.StatusCode)
	}
}

func TestAPIError404(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(404)
		json.NewEncoder(w).Encode(map[string]any{
			"error": map[string]any{
				"code":    "not_found",
				"message": "resource not found",
			},
		})
	})
	defer srv.Close()

	_, err := client.GetProjection(context.Background(), "nonexistent")
	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T", err)
	}
	if !apiErr.IsNotFound() {
		t.Errorf("expected IsNotFound, got status %d", apiErr.StatusCode)
	}
}

func TestAPIError429(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(429)
		json.NewEncoder(w).Encode(map[string]any{
			"error": map[string]any{
				"code":    "rate_limited",
				"message": "too many requests",
			},
		})
	})
	defer srv.Close()

	_, err := client.Health(context.Background())
	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("expected *APIError, got %T", err)
	}
	if !apiErr.IsRateLimited() {
		t.Errorf("expected IsRateLimited, got status %d", apiErr.StatusCode)
	}
}

func TestQueryWithAllOptions(t *testing.T) {
	srv, client := setupTestServer(func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()
		if q.Get("event_type") != "order.placed" {
			t.Errorf("expected event_type=order.placed, got %s", q.Get("event_type"))
		}
		if q.Get("entity_id") != "order-456" {
			t.Errorf("expected entity_id=order-456, got %s", q.Get("entity_id"))
		}
		if q.Get("start") != "2026-01-01T00:00:00Z" {
			t.Errorf("expected start=2026-01-01T00:00:00Z, got %s", q.Get("start"))
		}
		if q.Get("end") != "2026-02-01T00:00:00Z" {
			t.Errorf("expected end=2026-02-01T00:00:00Z, got %s", q.Get("end"))
		}
		if q.Get("limit") != "50" {
			t.Errorf("expected limit=50, got %s", q.Get("limit"))
		}
		if q.Get("offset") != "10" {
			t.Errorf("expected offset=10, got %s", q.Get("offset"))
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]any{
			"data": map[string]any{
				"events": []any{},
				"count":  0,
			},
		})
	})
	defer srv.Close()

	result, err := client.Query(context.Background(), QueryOptions{
		EventType: "order.placed",
		EntityID:  "order-456",
		Start:     "2026-01-01T00:00:00Z",
		End:       "2026-02-01T00:00:00Z",
		Limit:     50,
		Offset:    10,
	})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Count != 0 {
		t.Errorf("expected count 0, got %d", result.Count)
	}
}

func TestAPIErrorMessage(t *testing.T) {
	err := &APIError{
		Code:       "unauthorized",
		Message:    "invalid api key",
		StatusCode: 401,
	}
	expected := "allsource: unauthorized (status 401): invalid api key"
	if err.Error() != expected {
		t.Errorf("expected %q, got %q", expected, err.Error())
	}
}
