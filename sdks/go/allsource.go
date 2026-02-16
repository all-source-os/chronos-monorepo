// Package allsource provides a Go client for the AllSource Event Store API.
package allsource

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

// Client is the AllSource Event Store API client.
type Client struct {
	apiKey  string
	baseURL string
	http    *http.Client
}

// New creates a new AllSource client.
func New(apiKey, baseURL string) *Client {
	return &Client{
		apiKey:  apiKey,
		baseURL: baseURL,
		http: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// NewWithHTTPClient creates a new AllSource client with a custom http.Client.
func NewWithHTTPClient(apiKey, baseURL string, httpClient *http.Client) *Client {
	return &Client{
		apiKey:  apiKey,
		baseURL: baseURL,
		http:    httpClient,
	}
}

func (c *Client) do(ctx context.Context, method, path string, body any) ([]byte, int, error) {
	var reqBody io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, 0, fmt.Errorf("marshal request body: %w", err)
		}
		reqBody = bytes.NewReader(b)
	}

	req, err := http.NewRequestWithContext(ctx, method, c.baseURL+path, reqBody)
	if err != nil {
		return nil, 0, fmt.Errorf("create request: %w", err)
	}
	req.Header.Set("Authorization", "Bearer "+c.apiKey)
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, 0, fmt.Errorf("execute request: %w", err)
	}
	defer resp.Body.Close()

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, resp.StatusCode, fmt.Errorf("read response body: %w", err)
	}

	if resp.StatusCode >= 400 {
		return nil, resp.StatusCode, parseAPIError(data, resp.StatusCode)
	}

	return data, resp.StatusCode, nil
}

func parseAPIError(data []byte, statusCode int) error {
	var errResp struct {
		Error struct {
			Code    string `json:"code"`
			Message string `json:"message"`
		} `json:"error"`
	}
	if err := json.Unmarshal(data, &errResp); err == nil && errResp.Error.Code != "" {
		return &APIError{
			Code:       errResp.Error.Code,
			Message:    errResp.Error.Message,
			StatusCode: statusCode,
		}
	}
	return &APIError{
		Code:       "api_error",
		Message:    string(data),
		StatusCode: statusCode,
	}
}

// Ingest sends a single event to the AllSource Event Store.
func (c *Client) Ingest(ctx context.Context, eventType, entityID string, data map[string]any) (*Event, error) {
	body := map[string]any{
		"event_type": eventType,
		"entity_id":  entityID,
		"payload":    data,
	}
	respData, _, err := c.do(ctx, http.MethodPost, "/api/events", body)
	if err != nil {
		return nil, err
	}

	var wrapper struct {
		Data Event `json:"data"`
	}
	if err := json.Unmarshal(respData, &wrapper); err != nil {
		// Try parsing directly as Event
		var event Event
		if err2 := json.Unmarshal(respData, &event); err2 != nil {
			return nil, fmt.Errorf("decode event response: %w", err)
		}
		return &event, nil
	}
	return &wrapper.Data, nil
}

// QueryOptions specifies filters for querying events.
type QueryOptions struct {
	EventType string
	EntityID  string
	Start     string // ISO-8601 timestamp
	End       string // ISO-8601 timestamp
	Limit     int
	Offset    int
}

// Query retrieves events matching the given filters.
func (c *Client) Query(ctx context.Context, opts QueryOptions) (*EventList, error) {
	params := url.Values{}
	if opts.EventType != "" {
		params.Set("event_type", opts.EventType)
	}
	if opts.EntityID != "" {
		params.Set("entity_id", opts.EntityID)
	}
	if opts.Start != "" {
		params.Set("start", opts.Start)
	}
	if opts.End != "" {
		params.Set("end", opts.End)
	}
	if opts.Limit > 0 {
		params.Set("limit", strconv.Itoa(opts.Limit))
	}
	if opts.Offset > 0 {
		params.Set("offset", strconv.Itoa(opts.Offset))
	}

	path := "/api/events"
	if len(params) > 0 {
		path += "?" + params.Encode()
	}

	respData, _, err := c.do(ctx, http.MethodGet, path, nil)
	if err != nil {
		return nil, err
	}

	var wrapper struct {
		Data json.RawMessage `json:"data"`
	}
	if err := json.Unmarshal(respData, &wrapper); err == nil && wrapper.Data != nil {
		var el EventList
		if err := json.Unmarshal(wrapper.Data, &el); err != nil {
			return nil, fmt.Errorf("decode event list: %w", err)
		}
		return &el, nil
	}

	var el EventList
	if err := json.Unmarshal(respData, &el); err != nil {
		return nil, fmt.Errorf("decode event list: %w", err)
	}
	return &el, nil
}

// GetProjections returns all projections.
func (c *Client) GetProjections(ctx context.Context) ([]Projection, error) {
	respData, _, err := c.do(ctx, http.MethodGet, "/api/projections", nil)
	if err != nil {
		return nil, err
	}

	var wrapper struct {
		Data []Projection `json:"data"`
	}
	if err := json.Unmarshal(respData, &wrapper); err == nil && wrapper.Data != nil {
		return wrapper.Data, nil
	}

	var projections []Projection
	if err := json.Unmarshal(respData, &projections); err != nil {
		return nil, fmt.Errorf("decode projections: %w", err)
	}
	return projections, nil
}

// GetProjection returns a single projection by name.
func (c *Client) GetProjection(ctx context.Context, name string) (*Projection, error) {
	respData, _, err := c.do(ctx, http.MethodGet, "/api/projections/"+url.PathEscape(name), nil)
	if err != nil {
		return nil, err
	}

	var wrapper struct {
		Data Projection `json:"data"`
	}
	if err := json.Unmarshal(respData, &wrapper); err == nil && wrapper.Data.Name != "" {
		return &wrapper.Data, nil
	}

	var p Projection
	if err := json.Unmarshal(respData, &p); err != nil {
		return nil, fmt.Errorf("decode projection: %w", err)
	}
	return &p, nil
}

// Health checks the API health endpoint.
func (c *Client) Health(ctx context.Context) (map[string]any, error) {
	respData, _, err := c.do(ctx, http.MethodGet, "/health", nil)
	if err != nil {
		return nil, err
	}

	var result map[string]any
	if err := json.Unmarshal(respData, &result); err != nil {
		return nil, fmt.Errorf("decode health response: %w", err)
	}
	return result, nil
}
