package clients

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"time"

	"github.com/go-resty/resty/v2"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/trace"
)

// CoreClient defines typed methods for communicating with the Core service (Rust, port 3900).
type CoreClient interface {
	// Tenant management
	CreateTenant(ctx context.Context, req CreateTenantRequest) (*TenantResponse, error)
	GetTenant(ctx context.Context, tenantID string) (*TenantResponse, error)
	ListTenants(ctx context.Context) (*ListTenantsResponse, error)
	ActivateTenant(ctx context.Context, tenantID string) (*TenantResponse, error)
	DeactivateTenant(ctx context.Context, tenantID string) (*TenantResponse, error)
	DeleteTenant(ctx context.Context, tenantID string) error
	GetTenantStats(ctx context.Context, tenantID string) (*TenantStatsResponse, error)
	UpdateTenantQuotas(ctx context.Context, tenantID string, req UpdateQuotasRequest) (*TenantResponse, error)

	// Operations (stubs for Phase 3)
	CreateSnapshot(ctx context.Context, req CreateSnapshotRequest) (*SnapshotResponse, error)
	TriggerCompaction(ctx context.Context, req CompactionRequest) (*CompactionResponse, error)
	GetCompactionStats(ctx context.Context) (*CompactionStatsResponse, error)
	StartReplay(ctx context.Context, req ReplayRequest) (*ReplayResponse, error)
	GetReplayProgress(ctx context.Context, replayID string) (*ReplayProgressResponse, error)
	CancelReplay(ctx context.Context, replayID string) error

	// Schema (stubs for Phase 4)
	RegisterSchema(ctx context.Context, req RegisterSchemaRequest) (*SchemaResponse, error)
	ListSchemas(ctx context.Context) (*ListSchemasResponse, error)
	ValidateEvent(ctx context.Context, req ValidateEventRequest) (*ValidationResponse, error)

	// Health
	HealthCheck(ctx context.Context) (*HealthResponse, error)
	GetStats(ctx context.Context) (*StatsResponse, error)
}

// --- Request types ---

type CreateTenantRequest struct {
	ID       string         `json:"id"`
	Name     string         `json:"name"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

type UpdateQuotasRequest struct {
	MaxEventsPerSecond int64 `json:"max_events_per_second,omitempty"`
	MaxStorageBytes    int64 `json:"max_storage_bytes,omitempty"`
	MaxStreams         int64 `json:"max_streams,omitempty"`
}

type CreateSnapshotRequest struct {
	TenantID string `json:"tenant_id,omitempty"`
}

type CompactionRequest struct {
	Force bool `json:"force,omitempty"`
}

type ReplayRequest struct {
	EntityID string     `json:"entity_id"`
	AsOf     *time.Time `json:"as_of,omitempty"`
}

type RegisterSchemaRequest struct {
	EventType string `json:"event_type"`
	Schema    any    `json:"schema"`
	Version   string `json:"version,omitempty"`
}

type ValidateEventRequest struct {
	EventType string `json:"event_type"`
	Data      any    `json:"data"`
}

// --- Response types ---

type TenantResponse struct {
	ID       string         `json:"id"`
	Name     string         `json:"name"`
	Status   string         `json:"status"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

type ListTenantsResponse struct {
	Tenants []TenantResponse `json:"tenants"`
	Total   int              `json:"total"`
}

type TenantStatsResponse struct {
	TenantID    string `json:"tenant_id"`
	EventCount  int64  `json:"event_count"`
	StorageUsed int64  `json:"storage_used"`
	StreamCount int64  `json:"stream_count"`
}

type SnapshotResponse struct {
	SnapshotID string `json:"snapshot_id"`
	Status     string `json:"status"`
}

type CompactionResponse struct {
	Status string `json:"status"`
}

type CompactionStatsResponse struct {
	LastRun        *time.Time `json:"last_run,omitempty"`
	TotalRuns      int64      `json:"total_runs"`
	SpaceReclaimed int64      `json:"space_reclaimed"`
}

type ReplayResponse struct {
	ReplayID string `json:"replay_id"`
	Status   string `json:"status"`
}

type ReplayProgressResponse struct {
	ReplayID string  `json:"replay_id"`
	Status   string  `json:"status"`
	Progress float64 `json:"progress"`
}

type SchemaResponse struct {
	EventType string `json:"event_type"`
	Schema    any    `json:"schema"`
	Version   string `json:"version"`
}

type ListSchemasResponse struct {
	Schemas []SchemaResponse `json:"schemas"`
	Total   int              `json:"total"`
}

type ValidationResponse struct {
	Valid  bool     `json:"valid"`
	Errors []string `json:"errors,omitempty"`
}

type HealthResponse struct {
	Status  string         `json:"status"`
	Version string         `json:"version,omitempty"`
	Details map[string]any `json:"details,omitempty"`
}

type StatsResponse struct {
	EventCount    int64          `json:"event_count"`
	StreamCount   int64          `json:"stream_count"`
	StorageBytes  int64          `json:"storage_bytes"`
	UptimeSeconds float64        `json:"uptime_seconds"`
	Extra         map[string]any `json:"extra,omitempty"`
}

// --- Implementation ---

const (
	defaultCoreURL   = "http://localhost:3900"
	defaultTimeout   = 30 * time.Second
	maxRetries       = 3
	retryWaitTime    = 500 * time.Millisecond
	retryMaxWaitTime = 5 * time.Second
	tracerName       = "core-client"
)

type coreClient struct {
	client *resty.Client
	tracer trace.Tracer
}

// NewCoreClient creates a CoreClient using CORE_SERVICE_URL and CORE_API_KEY env vars.
func NewCoreClient() CoreClient {
	baseURL := os.Getenv("CORE_SERVICE_URL")
	if baseURL == "" {
		baseURL = defaultCoreURL
	}

	apiKey := os.Getenv("CORE_API_KEY")

	transport := &http.Transport{
		MaxIdleConns:        100,
		MaxIdleConnsPerHost: 100,
		IdleConnTimeout:     90 * time.Second,
	}

	client := resty.NewWithClient(&http.Client{Transport: transport}).
		SetBaseURL(baseURL).
		SetTimeout(defaultTimeout).
		SetRetryCount(maxRetries).
		SetRetryWaitTime(retryWaitTime).
		SetRetryMaxWaitTime(retryMaxWaitTime).
		AddRetryCondition(func(r *resty.Response, err error) bool {
			if err != nil {
				return true
			}
			return r.StatusCode() >= 500
		})

	if apiKey != "" {
		client.SetHeader("X-API-Key", apiKey)
	}

	return &coreClient{
		client: client,
		tracer: otel.Tracer(tracerName),
	}
}

// NewCoreClientWithURL creates a CoreClient with an explicit base URL and API key.
func NewCoreClientWithURL(baseURL, apiKey string) CoreClient {
	transport := &http.Transport{
		MaxIdleConns:        100,
		MaxIdleConnsPerHost: 100,
		IdleConnTimeout:     90 * time.Second,
	}

	client := resty.NewWithClient(&http.Client{Transport: transport}).
		SetBaseURL(baseURL).
		SetTimeout(defaultTimeout).
		SetRetryCount(maxRetries).
		SetRetryWaitTime(retryWaitTime).
		SetRetryMaxWaitTime(retryMaxWaitTime).
		AddRetryCondition(func(r *resty.Response, err error) bool {
			if err != nil {
				return true
			}
			return r.StatusCode() >= 500
		})

	if apiKey != "" {
		client.SetHeader("X-API-Key", apiKey)
	}

	return &coreClient{
		client: client,
		tracer: otel.Tracer(tracerName),
	}
}

// --- Tenant methods ---

func (c *coreClient) CreateTenant(ctx context.Context, req CreateTenantRequest) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "CreateTenant")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/tenants")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) GetTenant(ctx context.Context, tenantID string) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "GetTenant", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/tenants/" + tenantID)

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ListTenants(ctx context.Context) (*ListTenantsResponse, error) {
	var result ListTenantsResponse
	ctx, span := c.startSpan(ctx, "ListTenants")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/tenants")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ActivateTenant(ctx context.Context, tenantID string) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "ActivateTenant", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Post("/api/v1/tenants/" + tenantID + "/activate")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) DeactivateTenant(ctx context.Context, tenantID string) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "DeactivateTenant", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Post("/api/v1/tenants/" + tenantID + "/deactivate")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) DeleteTenant(ctx context.Context, tenantID string) error {
	ctx, span := c.startSpan(ctx, "DeleteTenant", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		Delete("/api/v1/tenants/" + tenantID)

	return c.handleError(span, resp, err)
}

func (c *coreClient) GetTenantStats(ctx context.Context, tenantID string) (*TenantStatsResponse, error) {
	var result TenantStatsResponse
	ctx, span := c.startSpan(ctx, "GetTenantStats", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/tenants/" + tenantID + "/stats")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) UpdateTenantQuotas(ctx context.Context, tenantID string, req UpdateQuotasRequest) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "UpdateTenantQuotas", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Put("/api/v1/tenants/" + tenantID + "/quotas")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

// --- Operations stubs (Phase 3) ---

func (c *coreClient) CreateSnapshot(ctx context.Context, req CreateSnapshotRequest) (*SnapshotResponse, error) {
	var result SnapshotResponse
	ctx, span := c.startSpan(ctx, "CreateSnapshot")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/ops/snapshot")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) TriggerCompaction(ctx context.Context, req CompactionRequest) (*CompactionResponse, error) {
	var result CompactionResponse
	ctx, span := c.startSpan(ctx, "TriggerCompaction")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/ops/compact")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) GetCompactionStats(ctx context.Context) (*CompactionStatsResponse, error) {
	var result CompactionStatsResponse
	ctx, span := c.startSpan(ctx, "GetCompactionStats")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/ops/compact/stats")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) StartReplay(ctx context.Context, req ReplayRequest) (*ReplayResponse, error) {
	var result ReplayResponse
	ctx, span := c.startSpan(ctx, "StartReplay")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/ops/replay")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) GetReplayProgress(ctx context.Context, replayID string) (*ReplayProgressResponse, error) {
	var result ReplayProgressResponse
	ctx, span := c.startSpan(ctx, "GetReplayProgress", attribute.String("replay.id", replayID))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/ops/replay/" + replayID)

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) CancelReplay(ctx context.Context, replayID string) error {
	ctx, span := c.startSpan(ctx, "CancelReplay", attribute.String("replay.id", replayID))
	defer span.End()

	resp, err := c.request(ctx).
		Delete("/api/v1/ops/replay/" + replayID)

	return c.handleError(span, resp, err)
}

// --- Schema stubs (Phase 4) ---

func (c *coreClient) RegisterSchema(ctx context.Context, req RegisterSchemaRequest) (*SchemaResponse, error) {
	var result SchemaResponse
	ctx, span := c.startSpan(ctx, "RegisterSchema")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/schemas")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ListSchemas(ctx context.Context) (*ListSchemasResponse, error) {
	var result ListSchemasResponse
	ctx, span := c.startSpan(ctx, "ListSchemas")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/schemas")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ValidateEvent(ctx context.Context, req ValidateEventRequest) (*ValidationResponse, error) {
	var result ValidationResponse
	ctx, span := c.startSpan(ctx, "ValidateEvent")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/schemas/validate")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

// --- Health methods ---

func (c *coreClient) HealthCheck(ctx context.Context) (*HealthResponse, error) {
	var result HealthResponse
	ctx, span := c.startSpan(ctx, "HealthCheck")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/health")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) GetStats(ctx context.Context) (*StatsResponse, error) {
	var result StatsResponse
	ctx, span := c.startSpan(ctx, "GetStats")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/stats")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

// --- Internal helpers ---

func (c *coreClient) startSpan(ctx context.Context, operation string, attrs ...attribute.KeyValue) (context.Context, trace.Span) {
	attrs = append(attrs,
		attribute.String("rpc.system", "http"),
		attribute.String("rpc.service", "chronos-core"),
		attribute.String("rpc.method", operation),
	)
	return c.tracer.Start(ctx, "CoreClient."+operation,
		trace.WithSpanKind(trace.SpanKindClient),
		trace.WithAttributes(attrs...),
	)
}

func (c *coreClient) request(ctx context.Context) *resty.Request {
	req := c.client.R().SetContext(ctx)

	// Inject OTEL trace context into outgoing request headers
	carrier := make(map[string]string)
	otel.GetTextMapPropagator().Inject(ctx, &mapCarrier{m: carrier})
	for k, v := range carrier {
		req.SetHeader(k, v)
	}

	return req
}

func (c *coreClient) handleError(span trace.Span, resp *resty.Response, err error) error {
	if err != nil {
		span.RecordError(err)
		span.SetAttributes(attribute.Bool("error", true))
		return fmt.Errorf("core request failed: %w", err)
	}

	if resp != nil {
		span.SetAttributes(
			attribute.Int("http.status_code", resp.StatusCode()),
			attribute.Int("http.response_size", len(resp.Body())),
		)

		if resp.StatusCode() >= 400 {
			span.SetAttributes(attribute.Bool("error", true))
			return fmt.Errorf("core returned status %d: %s", resp.StatusCode(), resp.String())
		}
	}

	return nil
}

// mapCarrier implements propagation.TextMapCarrier for injecting trace context.
type mapCarrier struct {
	m map[string]string
}

func (mc *mapCarrier) Get(key string) string {
	return mc.m[key]
}

func (mc *mapCarrier) Set(key, value string) {
	mc.m[key] = value
}

func (mc *mapCarrier) Keys() []string {
	keys := make([]string, 0, len(mc.m))
	for k := range mc.m {
		keys = append(keys, k)
	}
	return keys
}
