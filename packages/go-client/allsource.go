// Package allsource provides a Go client for the AllSource event store.
package allsource

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

const defaultTimeout = 30 * time.Second

// Config holds the configuration for an AllSource client.
type Config struct {
	// BaseURL is the AllSource Query Service URL (e.g., "https://allsource-query.fly.dev").
	BaseURL string
	// APIKey is the API key for authentication (sent as X-API-Key header).
	APIKey string
	// Timeout is the HTTP request timeout. Defaults to 30s.
	Timeout time.Duration
	// HTTPClient is an optional custom HTTP client. If nil, a default client with the configured timeout is used.
	HTTPClient *http.Client
}

// Client is the AllSource API client.
type Client struct {
	baseURL    string
	apiKey     string
	httpClient *http.Client
}

// NewClient creates a new AllSource client.
func NewClient(cfg Config) (*Client, error) {
	if cfg.BaseURL == "" {
		return nil, fmt.Errorf("allsource: BaseURL is required")
	}
	if cfg.APIKey == "" {
		return nil, fmt.Errorf("allsource: APIKey is required")
	}

	baseURL := strings.TrimRight(cfg.BaseURL, "/")

	httpClient := cfg.HTTPClient
	if httpClient == nil {
		timeout := cfg.Timeout
		if timeout == 0 {
			timeout = defaultTimeout
		}
		httpClient = &http.Client{Timeout: timeout}
	}

	return &Client{
		baseURL:    baseURL,
		apiKey:     cfg.APIKey,
		httpClient: httpClient,
	}, nil
}

// IngestEventInput represents an event to ingest.
type IngestEventInput struct {
	// EventType is the type of event (e.g., "user.signup", "order.placed").
	EventType string `json:"event_type"`
	// EntityID is the entity this event belongs to (e.g., user ID, order ID).
	EntityID string `json:"entity_id"`
	// Payload is the arbitrary JSON payload for the event.
	Payload map[string]any `json:"payload"`
	// Metadata is optional metadata (e.g., source, version, ip).
	Metadata map[string]any `json:"metadata,omitempty"`
}

// Event represents a stored event returned from AllSource.
type Event struct {
	ID        string         `json:"id"`
	EventType string         `json:"event_type"`
	EntityID  string         `json:"entity_id"`
	Payload   map[string]any `json:"payload"`
	Metadata  map[string]any `json:"metadata"`
	Timestamp string         `json:"timestamp"`
	StreamID  string         `json:"stream_id,omitempty"`
}

// QueryEventsParams holds optional filters for querying events.
type QueryEventsParams struct {
	EntityID  string
	EventType string
	Limit     int
	Offset    int
	StartTime string
	EndTime   string
}

// QueryEventsResponse is the response from querying events.
type QueryEventsResponse struct {
	Events []Event `json:"events"`
	Count  int     `json:"count"`
}

// HealthResponse is the response from the health endpoint.
type HealthResponse struct {
	Status string         `json:"status"`
	Extra  map[string]any `json:"-"`
}

// UnmarshalJSON implements custom JSON unmarshalling to capture extra fields.
func (h *HealthResponse) UnmarshalJSON(data []byte) error {
	var raw map[string]any
	if err := json.Unmarshal(data, &raw); err != nil {
		return err
	}
	if s, ok := raw["status"].(string); ok {
		h.Status = s
	}
	delete(raw, "status")
	h.Extra = raw
	return nil
}

// Error is returned when the AllSource API returns a non-2xx response.
type Error struct {
	// Message is a human-readable error description.
	Message string
	// Status is the HTTP status code.
	Status int
	// Body is the response body (parsed JSON or raw string).
	Body any
}

func (e *Error) Error() string {
	return e.Message
}

// IngestEvent ingests a single event into AllSource.
func (c *Client) IngestEvent(ctx context.Context, input IngestEventInput) (*Event, error) {
	var event Event
	if err := c.doRequest(ctx, http.MethodPost, "/api/events", input, &event); err != nil {
		return nil, err
	}
	return &event, nil
}

// QueryEvents queries events with optional filters.
func (c *Client) QueryEvents(ctx context.Context, params *QueryEventsParams) (*QueryEventsResponse, error) {
	path := "/api/events"
	if params != nil {
		q := url.Values{}
		if params.EntityID != "" {
			q.Set("entity_id", params.EntityID)
		}
		if params.EventType != "" {
			q.Set("event_type", params.EventType)
		}
		if params.Limit > 0 {
			q.Set("limit", fmt.Sprintf("%d", params.Limit))
		}
		if params.Offset > 0 {
			q.Set("offset", fmt.Sprintf("%d", params.Offset))
		}
		if params.StartTime != "" {
			q.Set("start_time", params.StartTime)
		}
		if params.EndTime != "" {
			q.Set("end_time", params.EndTime)
		}
		if encoded := q.Encode(); encoded != "" {
			path += "?" + encoded
		}
	}

	var resp QueryEventsResponse
	if err := c.doRequest(ctx, http.MethodGet, path, nil, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

// GetHealth checks the health of the AllSource service.
func (c *Client) GetHealth(ctx context.Context) (*HealthResponse, error) {
	var resp HealthResponse
	if err := c.doRequest(ctx, http.MethodGet, "/api/health", nil, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

func (c *Client) doRequest(ctx context.Context, method, path string, body any, result any) error {
	reqURL := c.baseURL + path

	var bodyReader io.Reader
	if body != nil {
		data, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("allsource: failed to marshal request body: %w", err)
		}
		bodyReader = bytes.NewReader(data)
	}

	req, err := http.NewRequestWithContext(ctx, method, reqURL, bodyReader)
	if err != nil {
		return fmt.Errorf("allsource: failed to create request: %w", err)
	}

	req.Header.Set("X-API-Key", c.apiKey)
	req.Header.Set("Accept", "application/json")
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("allsource: request failed: %w", err)
	}
	defer resp.Body.Close()

	respBody, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("allsource: failed to read response body: %w", err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		var parsed any
		if err := json.Unmarshal(respBody, &parsed); err != nil {
			parsed = string(respBody)
		}
		return &Error{
			Message: fmt.Sprintf("AllSource API error: %d %s", resp.StatusCode, resp.Status),
			Status:  resp.StatusCode,
			Body:    parsed,
		}
	}

	if result != nil {
		if err := json.Unmarshal(respBody, result); err != nil {
			return fmt.Errorf("allsource: failed to parse response: %w", err)
		}
	}

	return nil
}
