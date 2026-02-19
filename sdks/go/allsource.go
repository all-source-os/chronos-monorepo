// Package allsource provides a Go client for the AllSource Event Store API.
package allsource

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

// RetryConfig controls exponential backoff retry behavior.
type RetryConfig struct {
	// MaxRetries is the maximum number of retry attempts (default 3).
	MaxRetries int
	// BaseDelay is the initial delay before the first retry (default 200ms).
	BaseDelay time.Duration
	// BackoffFactor is the multiplier applied to the delay after each retry (default 2.0).
	BackoffFactor float64
	// MaxDelay is the maximum delay between retries (default 10s).
	MaxDelay time.Duration
}

// DefaultRetryConfig returns the default retry configuration.
func DefaultRetryConfig() RetryConfig {
	return RetryConfig{
		MaxRetries:    3,
		BaseDelay:     200 * time.Millisecond,
		BackoffFactor: 2.0,
		MaxDelay:      10 * time.Second,
	}
}

// Option is a functional option for configuring the Client.
type Option func(*Client)

// WithRetry configures retry behavior with exponential backoff.
func WithRetry(cfg RetryConfig) Option {
	return func(c *Client) {
		c.retry = &cfg
	}
}

// WithCircuitBreaker configures a circuit breaker that opens after threshold
// consecutive failures and recovers after the given duration.
func WithCircuitBreaker(threshold int, recovery time.Duration) Option {
	return func(c *Client) {
		c.cb = NewCircuitBreaker(threshold, recovery)
	}
}

// WithHTTPClient sets a custom *http.Client for the AllSource client.
func WithHTTPClient(hc *http.Client) Option {
	return func(c *Client) {
		c.http = hc
	}
}

// WithTimeout sets the HTTP client timeout.
func WithTimeout(d time.Duration) Option {
	return func(c *Client) {
		c.http.Timeout = d
	}
}

// Client is the AllSource Event Store API client.
type Client struct {
	apiKey  string
	baseURL string
	http    *http.Client
	retry   *RetryConfig
	cb      *CircuitBreaker
}

// New creates a new AllSource client. Options are applied in order after defaults.
func New(apiKey, baseURL string, opts ...Option) *Client {
	c := &Client{
		apiKey:  apiKey,
		baseURL: baseURL,
		http: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// NewWithHTTPClient creates a new AllSource client with a custom http.Client.
// Deprecated: use New with WithHTTPClient instead.
func NewWithHTTPClient(apiKey, baseURL string, httpClient *http.Client) *Client {
	return &Client{
		apiKey:  apiKey,
		baseURL: baseURL,
		http:    httpClient,
	}
}

// retryableStatusCodes are HTTP status codes that are safe to retry.
var retryableStatusCodes = map[int]bool{
	408: true,
	429: true,
	500: true,
	502: true,
	503: true,
	504: true,
}

func (c *Client) do(ctx context.Context, method, path string, body any) ([]byte, int, error) {
	// Check circuit breaker before attempting the request.
	if c.cb != nil && !c.cb.Allow() {
		return nil, 0, ErrCircuitOpen
	}

	maxAttempts := 1
	var retryCfg RetryConfig
	if c.retry != nil {
		retryCfg = *c.retry
		maxAttempts = retryCfg.MaxRetries + 1
	}

	var lastErr error
	var lastStatus int

	for attempt := 0; attempt < maxAttempts; attempt++ {
		if attempt > 0 {
			delay := time.Duration(float64(retryCfg.BaseDelay) * math.Pow(retryCfg.BackoffFactor, float64(attempt-1)))
			if delay > retryCfg.MaxDelay {
				delay = retryCfg.MaxDelay
			}
			select {
			case <-ctx.Done():
				return nil, 0, ctx.Err()
			case <-time.After(delay):
			}
		}

		data, status, err := c.doOnce(ctx, method, path, body)
		if err == nil {
			if c.cb != nil {
				c.cb.RecordSuccess()
			}
			return data, status, nil
		}

		lastErr = err
		lastStatus = status

		// Determine if this error is retryable.
		if !c.shouldRetry(err, status) {
			if c.cb != nil {
				// Non-retryable client errors (4xx except 408/429) are not circuit breaker failures.
				if status >= 500 || status == 408 || status == 429 {
					c.cb.RecordFailure()
				}
			}
			return nil, lastStatus, lastErr
		}

		if c.cb != nil {
			c.cb.RecordFailure()
		}
	}

	return nil, lastStatus, lastErr
}

func (c *Client) shouldRetry(err error, status int) bool {
	if c.retry == nil {
		return false
	}

	// Transport errors (no status code) are retryable.
	var apiErr *APIError
	if errors.As(err, &apiErr) {
		return retryableStatusCodes[apiErr.StatusCode]
	}

	// Non-APIError means transport/network failure; retryable.
	if status == 0 {
		return true
	}

	return retryableStatusCodes[status]
}

func (c *Client) doOnce(ctx context.Context, method, path string, body any) ([]byte, int, error) {
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
	req.Header.Set("X-API-Key", c.apiKey)
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

// IngestResponse is the response from ingesting a single event.
type IngestResponse struct {
	EventID   string `json:"event_id"`
	Timestamp string `json:"timestamp"`
}

// BatchIngestResponse is the response from batch ingesting events.
type BatchIngestResponse struct {
	Total    int              `json:"total"`
	Ingested int              `json:"ingested"`
	Events   []IngestResponse `json:"events"`
}

// Ingest sends a single event to the AllSource Event Store.
func (c *Client) Ingest(ctx context.Context, eventType, entityID string, data map[string]any) (*IngestResponse, error) {
	body := map[string]any{
		"event_type": eventType,
		"entity_id":  entityID,
		"payload":    data,
	}
	respData, _, err := c.do(ctx, http.MethodPost, "/api/v1/events", body)
	if err != nil {
		return nil, err
	}

	var resp IngestResponse
	if err := json.Unmarshal(respData, &resp); err != nil {
		return nil, fmt.Errorf("decode ingest response: %w", err)
	}
	return &resp, nil
}

// IngestBatch sends multiple events in a single request.
func (c *Client) IngestBatch(ctx context.Context, events []map[string]any) (*BatchIngestResponse, error) {
	body := map[string]any{
		"events": events,
	}
	respData, _, err := c.do(ctx, http.MethodPost, "/api/v1/events/batch", body)
	if err != nil {
		return nil, err
	}

	var resp BatchIngestResponse
	if err := json.Unmarshal(respData, &resp); err != nil {
		return nil, fmt.Errorf("decode batch ingest response: %w", err)
	}
	return &resp, nil
}

// QueryOptions specifies filters for querying events.
type QueryOptions struct {
	EventType string
	EntityID  string
	Start     string // ISO-8601 timestamp, maps to "since" query param
	End       string // ISO-8601 timestamp, maps to "until" query param
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
		params.Set("since", opts.Start)
	}
	if opts.End != "" {
		params.Set("until", opts.End)
	}
	if opts.Limit > 0 {
		params.Set("limit", strconv.Itoa(opts.Limit))
	}
	if opts.Offset > 0 {
		params.Set("offset", strconv.Itoa(opts.Offset))
	}

	path := "/api/v1/events/query"
	if len(params) > 0 {
		path += "?" + params.Encode()
	}

	respData, _, err := c.do(ctx, http.MethodGet, path, nil)
	if err != nil {
		return nil, err
	}

	var el EventList
	if err := json.Unmarshal(respData, &el); err != nil {
		return nil, fmt.Errorf("decode event list: %w", err)
	}
	return &el, nil
}

// ProjectionList is a list of projections with a total.
type ProjectionList struct {
	Projections []Projection `json:"projections"`
	Total       int          `json:"total"`
}

// GetProjections returns all projections.
func (c *Client) GetProjections(ctx context.Context) (*ProjectionList, error) {
	respData, _, err := c.do(ctx, http.MethodGet, "/api/v1/projections", nil)
	if err != nil {
		return nil, err
	}

	var pl ProjectionList
	if err := json.Unmarshal(respData, &pl); err != nil {
		return nil, fmt.Errorf("decode projections: %w", err)
	}
	return &pl, nil
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
