// Package clients provides typed HTTP clients for communicating with external services.
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
	UpdateTenantMetadata(ctx context.Context, tenantID string, metadata map[string]any) (*TenantResponse, error)

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

	// Audit
	LogAuditEvent(ctx context.Context, req AuditEventRequest) error
	QueryAuditEvents(ctx context.Context, params AuditQueryParams) (*AuditEventsResponse, error)

	// Config
	SetConfig(ctx context.Context, req SetConfigRequest) error
	GetConfig(ctx context.Context, key string) (*ConfigEntryResponse, error)
	ListConfigs(ctx context.Context) (*ListConfigsResponse, error)
	UpdateConfig(ctx context.Context, key string, req UpdateConfigEntryRequest) (*ConfigEntryResponse, error)
	DeleteConfig(ctx context.Context, key string) error

	// Events
	IngestEvent(ctx context.Context, req IngestEventRequest) (*IngestEventResponse, error)
	QueryEvents(ctx context.Context, req QueryEventsRequest) (*QueryEventsResponse, error)

	// Health
	HealthCheck(ctx context.Context) (*HealthResponse, error)
	GetStats(ctx context.Context) (*StatsResponse, error)

	// Auth — API key provisioning (admin operations)
	CreateCoreAPIKey(ctx context.Context, req CreateCoreAPIKeyRequest) (*CreateCoreAPIKeyResponse, error)
	ListCoreAPIKeys(ctx context.Context, tenantID string) ([]CoreAPIKeyInfo, error)
	RevokeAPIKey(ctx context.Context, keyID string) error
}

// --- Request types ---

// CreateTenantRequest is the request body for creating a tenant in Core.
type CreateTenantRequest struct {
	ID       string         `json:"id"`
	Name     string         `json:"name"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

// UpdateQuotasRequest is the request body for updating tenant quotas.
type UpdateQuotasRequest struct {
	MaxEventsPerSecond int64 `json:"max_events_per_second,omitempty"`
	MaxStorageBytes    int64 `json:"max_storage_bytes,omitempty"`
	MaxStreams         int64 `json:"max_streams,omitempty"`
}

// CreateSnapshotRequest is the request body for creating a snapshot.
type CreateSnapshotRequest struct {
	TenantID string `json:"tenant_id,omitempty"`
}

// CompactionRequest is the request body for triggering compaction.
type CompactionRequest struct {
	Force bool `json:"force,omitempty"`
}

// ReplayRequest is the request body for starting a replay.
type ReplayRequest struct {
	EntityID string     `json:"entity_id"`
	AsOf     *time.Time `json:"as_of,omitempty"`
}

// RegisterSchemaRequest is the request body for registering a schema.
type RegisterSchemaRequest struct {
	EventType string `json:"event_type"`
	Schema    any    `json:"schema"`
	Version   string `json:"version,omitempty"`
}

// ValidateEventRequest is the request body for validating an event against a schema.
type ValidateEventRequest struct {
	EventType string `json:"event_type"`
	Data      any    `json:"data"`
}

// IngestEventRequest is the request body for ingesting an event into Core.
type IngestEventRequest struct {
	EventType string         `json:"event_type"`
	EntityID  string         `json:"entity_id"`
	Payload   map[string]any `json:"payload,omitempty"`
	TenantID  string         `json:"tenant_id,omitempty"`
}

// IngestEventResponse is the response from ingesting an event.
type IngestEventResponse struct {
	EventID string `json:"event_id,omitempty"`
	ID      string `json:"id,omitempty"`
}

// QueryEventsRequest is the request for querying events from Core.
type QueryEventsRequest struct {
	EntityID  string `json:"entity_id,omitempty"`
	EventType string `json:"event_type,omitempty"` // prefix match supported
	TenantID  string `json:"tenant_id,omitempty"`  // required post-auth-skip cutover
	Since     string `json:"since,omitempty"`      // RFC3339
	Until     string `json:"until,omitempty"`      // RFC3339
	Limit     int    `json:"limit,omitempty"`
	Offset    int    `json:"offset,omitempty"`
}

// QueryEventsResponse is the response from querying events.
type QueryEventsResponse struct {
	Events []EventEntry `json:"events"`
	Count  int          `json:"count"`
}

// EventEntry represents a single event from Core's query response.
type EventEntry struct {
	ID        string         `json:"id"`
	EventType string         `json:"event_type"`
	EntityID  string         `json:"entity_id"`
	Timestamp string         `json:"timestamp"`
	Payload   map[string]any `json:"payload,omitempty"`
}

// --- Response types ---

// TenantResponse represents a tenant from Core.
type TenantResponse struct {
	ID       string         `json:"id"`
	Name     string         `json:"name"`
	Status   string         `json:"status"`
	IsDemo   bool           `json:"is_demo,omitempty"`
	Metadata map[string]any `json:"metadata,omitempty"`
}

// ListTenantsResponse wraps a list of tenants from Core.
type ListTenantsResponse struct {
	Tenants []TenantResponse `json:"tenants"`
	Total   int              `json:"total"`
}

// TenantStatsResponse contains tenant usage statistics.
type TenantStatsResponse struct {
	TenantID    string `json:"tenant_id"`
	EventCount  int64  `json:"event_count"`
	StorageUsed int64  `json:"storage_used"`
	StreamCount int64  `json:"stream_count"`
}

// SnapshotResponse represents a snapshot operation result.
type SnapshotResponse struct {
	SnapshotID string `json:"snapshot_id"`
	Status     string `json:"status"`
}

// CompactionResponse represents a compaction operation result.
type CompactionResponse struct {
	Status string `json:"status"`
}

// CompactionStatsResponse contains compaction statistics.
type CompactionStatsResponse struct {
	LastRun        *time.Time `json:"last_run,omitempty"`
	TotalRuns      int64      `json:"total_runs"`
	SpaceReclaimed int64      `json:"space_reclaimed"`
}

// ReplayResponse represents a replay operation result.
type ReplayResponse struct {
	ReplayID string `json:"replay_id"`
	Status   string `json:"status"`
}

// ReplayProgressResponse contains replay progress information.
type ReplayProgressResponse struct {
	ReplayID string  `json:"replay_id"`
	Status   string  `json:"status"`
	Progress float64 `json:"progress"`
}

// SchemaResponse represents a registered schema.
type SchemaResponse struct {
	EventType string `json:"event_type"`
	Schema    any    `json:"schema"`
	Version   string `json:"version"`
}

// ListSchemasResponse wraps a list of schemas from Core.
type ListSchemasResponse struct {
	Schemas []SchemaResponse `json:"schemas"`
	Total   int              `json:"total"`
}

// ValidationResponse contains schema validation results.
type ValidationResponse struct {
	Valid  bool     `json:"valid"`
	Errors []string `json:"errors,omitempty"`
}

// HealthResponse contains Core health check results.
type HealthResponse struct {
	Status  string         `json:"status"`
	Version string         `json:"version,omitempty"`
	Details map[string]any `json:"details,omitempty"`
}

// StatsResponse contains Core statistics.
type StatsResponse struct {
	EventCount    int64          `json:"event_count"`
	StreamCount   int64          `json:"stream_count"`
	StorageBytes  int64          `json:"storage_bytes"`
	UptimeSeconds float64        `json:"uptime_seconds"`
	Extra         map[string]any `json:"extra,omitempty"`
}

// --- Audit types ---

// AuditEventRequest is the request body for logging an audit event.
type AuditEventRequest struct {
	TenantID     string         `json:"tenant_id"`
	Action       string         `json:"action"`
	ActorType    string         `json:"actor_type"`
	ActorID      string         `json:"actor_id"`
	ActorName    string         `json:"actor_name"`
	Outcome      string         `json:"outcome,omitempty"`
	ResourceType string         `json:"resource_type,omitempty"`
	ResourceID   string         `json:"resource_id,omitempty"`
	IPAddress    string         `json:"ip_address,omitempty"`
	UserAgent    string         `json:"user_agent,omitempty"`
	ErrorMessage string         `json:"error_message,omitempty"`
	Metadata     map[string]any `json:"metadata,omitempty"`
}

// AuditQueryParams contains query parameters for audit event searches.
type AuditQueryParams struct {
	TenantID     string `json:"tenant_id,omitempty"`
	UserID       string `json:"user_id,omitempty"`
	Action       string `json:"action,omitempty"`
	Start        string `json:"start,omitempty"`
	End          string `json:"end,omitempty"`
	SecurityOnly bool   `json:"security_only,omitempty"`
	Limit        int    `json:"limit,omitempty"`
	Offset       int    `json:"offset,omitempty"`
}

// AuditEventItem represents a single audit event from Core.
type AuditEventItem struct {
	ID           string         `json:"id"`
	TenantID     string         `json:"tenant_id"`
	Timestamp    string         `json:"timestamp"`
	Action       string         `json:"action"`
	Actor        map[string]any `json:"actor"`
	Outcome      string         `json:"outcome"`
	ResourceType string         `json:"resource_type,omitempty"`
	ResourceID   string         `json:"resource_id,omitempty"`
	IPAddress    string         `json:"ip_address,omitempty"`
	ErrorMessage string         `json:"error_message,omitempty"`
	Metadata     map[string]any `json:"metadata,omitempty"`
}

// AuditEventsResponse wraps a list of audit events from Core.
type AuditEventsResponse struct {
	Events []AuditEventItem `json:"events"`
	Total  int              `json:"total"`
}

// --- Auth / API key types ---

// CreateCoreAPIKeyRequest is the request body for creating a Core API key on behalf of a tenant.
type CreateCoreAPIKeyRequest struct {
	Name     string `json:"name"`
	TenantID string `json:"tenant_id"`
	Role     string `json:"role,omitempty"`
}

// CreateCoreAPIKeyResponse is the response from Core when an API key is created.
type CreateCoreAPIKeyResponse struct {
	ID  string `json:"id"`
	Key string `json:"key"`
}

// CoreAPIKeyInfo is summary info about a Core API key.
type CoreAPIKeyInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	TenantID string `json:"tenant_id"`
	Active   bool   `json:"active"`
}

// --- Config types ---

// SetConfigRequest is the request body for setting a config entry.
type SetConfigRequest struct {
	Key       string `json:"key"`
	Value     any    `json:"value"`
	ChangedBy string `json:"changed_by,omitempty"`
}

// UpdateConfigEntryRequest is the request body for updating a config entry.
type UpdateConfigEntryRequest struct {
	Value     any    `json:"value"`
	ChangedBy string `json:"changed_by,omitempty"`
}

// ConfigEntryResponse represents a config entry from Core.
type ConfigEntryResponse struct {
	Key       string `json:"key"`
	Value     any    `json:"value"`
	UpdatedAt string `json:"updated_at"`
	UpdatedBy string `json:"updated_by,omitempty"`
}

// ListConfigsResponse wraps a list of config entries from Core.
type ListConfigsResponse struct {
	Configs []ConfigEntryResponse `json:"configs"`
	Total   int                   `json:"total"`
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

// NewCoreClientWithJWT creates a CoreClient that authenticates with a Bearer JWT.
// Use this for service-to-service calls where the caller holds an admin JWT
// (e.g. the control plane service token) rather than a static API key.
func NewCoreClientWithJWT(baseURL, bearerToken string) CoreClient {
	transport := &http.Transport{
		MaxIdleConns:        100,
		MaxIdleConnsPerHost: 100,
		IdleConnTimeout:     90 * time.Second,
	}

	// Preserve Authorization header across redirects (Go strips it by default).
	httpClient := &http.Client{
		Transport: transport,
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			if len(via) >= 10 {
				return fmt.Errorf("stopped after 10 redirects")
			}
			if auth := via[0].Header.Get("Authorization"); auth != "" {
				// Intentional: Core is an internal service, we control both
				// endpoints, and the redirect target is constrained by
				// baseURL above. Stripping auth would break service-to-service
				// calls after any redirect.
				req.Header.Set("Authorization", auth) //nolint:gosec // G119
			}
			return nil
		},
	}

	client := resty.NewWithClient(httpClient).
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

	if bearerToken != "" {
		client.SetAuthToken(bearerToken)
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
	// Core returns a bare JSON array of tenants, not a {tenants,total} object.
	var arr []TenantResponse
	ctx, span := c.startSpan(ctx, "ListTenants")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&arr).
		Get("/api/v1/tenants")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &ListTenantsResponse{Tenants: arr, Total: len(arr)}, nil
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

func (c *coreClient) UpdateTenantMetadata(ctx context.Context, tenantID string, metadata map[string]any) (*TenantResponse, error) {
	var result TenantResponse
	ctx, span := c.startSpan(ctx, "UpdateTenantMetadata", attribute.String("tenant.id", tenantID))
	defer span.End()

	body := map[string]any{"metadata": metadata}
	resp, err := c.request(ctx).
		SetBody(body).
		SetResult(&result).
		Patch("/api/v1/tenants/" + tenantID)

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

// --- Event methods ---

func (c *coreClient) IngestEvent(ctx context.Context, req IngestEventRequest) (*IngestEventResponse, error) {
	var result IngestEventResponse
	ctx, span := c.startSpan(ctx, "IngestEvent")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/events")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) QueryEvents(ctx context.Context, req QueryEventsRequest) (*QueryEventsResponse, error) {
	var result QueryEventsResponse
	ctx, span := c.startSpan(ctx, "QueryEvents")
	defer span.End()

	r := c.request(ctx).SetResult(&result)
	if req.EntityID != "" {
		r.SetQueryParam("entity_id", req.EntityID)
	}
	if req.EventType != "" {
		r.SetQueryParam("event_type", req.EventType)
	}
	if req.TenantID != "" {
		r.SetQueryParam("tenant_id", req.TenantID)
	}
	if req.Since != "" {
		r.SetQueryParam("since", req.Since)
	}
	if req.Until != "" {
		r.SetQueryParam("until", req.Until)
	}
	if req.Limit > 0 {
		r.SetQueryParam("limit", fmt.Sprintf("%d", req.Limit))
	}
	if req.Offset > 0 {
		r.SetQueryParam("offset", fmt.Sprintf("%d", req.Offset))
	}

	resp, err := r.Get("/api/v1/events/query")
	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

// --- Audit methods ---

func (c *coreClient) LogAuditEvent(ctx context.Context, req AuditEventRequest) error {
	ctx, span := c.startSpan(ctx, "LogAuditEvent")
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		Post("/api/v1/audit/events")

	return c.handleError(span, resp, err)
}

func (c *coreClient) QueryAuditEvents(ctx context.Context, params AuditQueryParams) (*AuditEventsResponse, error) {
	var result AuditEventsResponse
	ctx, span := c.startSpan(ctx, "QueryAuditEvents")
	defer span.End()

	req := c.request(ctx).SetResult(&result)

	queryParams := map[string]string{}
	if params.TenantID != "" {
		queryParams["tenant_id"] = params.TenantID
	}
	if params.UserID != "" {
		queryParams["user_id"] = params.UserID
	}
	if params.Action != "" {
		queryParams["action"] = params.Action
	}
	if params.Start != "" {
		queryParams["start"] = params.Start
	}
	if params.End != "" {
		queryParams["end"] = params.End
	}
	if params.SecurityOnly {
		queryParams["security_only"] = "true"
	}
	if params.Limit > 0 {
		queryParams["limit"] = fmt.Sprintf("%d", params.Limit)
	}
	if params.Offset > 0 {
		queryParams["offset"] = fmt.Sprintf("%d", params.Offset)
	}
	req.SetQueryParams(queryParams)

	resp, err := req.Get("/api/v1/audit/events")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

// --- Config methods ---

func (c *coreClient) SetConfig(ctx context.Context, req SetConfigRequest) error {
	ctx, span := c.startSpan(ctx, "SetConfig", attribute.String("config.key", req.Key))
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		Post("/api/v1/config")

	return c.handleError(span, resp, err)
}

func (c *coreClient) GetConfig(ctx context.Context, key string) (*ConfigEntryResponse, error) {
	var result ConfigEntryResponse
	ctx, span := c.startSpan(ctx, "GetConfig", attribute.String("config.key", key))
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/config/" + key)

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ListConfigs(ctx context.Context) (*ListConfigsResponse, error) {
	var result ListConfigsResponse
	ctx, span := c.startSpan(ctx, "ListConfigs")
	defer span.End()

	resp, err := c.request(ctx).
		SetResult(&result).
		Get("/api/v1/config")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) UpdateConfig(ctx context.Context, key string, req UpdateConfigEntryRequest) (*ConfigEntryResponse, error) {
	var result ConfigEntryResponse
	ctx, span := c.startSpan(ctx, "UpdateConfig", attribute.String("config.key", key))
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Put("/api/v1/config/" + key)

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) DeleteConfig(ctx context.Context, key string) error {
	ctx, span := c.startSpan(ctx, "DeleteConfig", attribute.String("config.key", key))
	defer span.End()

	resp, err := c.request(ctx).
		Delete("/api/v1/config/" + key)

	return c.handleError(span, resp, err)
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

// --- Auth / API key methods ---

func (c *coreClient) CreateCoreAPIKey(ctx context.Context, req CreateCoreAPIKeyRequest) (*CreateCoreAPIKeyResponse, error) {
	var result CreateCoreAPIKeyResponse
	ctx, span := c.startSpan(ctx, "CreateCoreAPIKey", attribute.String("tenant.id", req.TenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetBody(req).
		SetResult(&result).
		Post("/api/v1/auth/api-keys")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return &result, nil
}

func (c *coreClient) ListCoreAPIKeys(ctx context.Context, tenantID string) ([]CoreAPIKeyInfo, error) {
	var result []CoreAPIKeyInfo
	ctx, span := c.startSpan(ctx, "ListCoreAPIKeys", attribute.String("tenant.id", tenantID))
	defer span.End()

	resp, err := c.request(ctx).
		SetQueryParam("tenant_id", tenantID).
		SetResult(&result).
		Get("/api/v1/auth/api-keys")

	if err := c.handleError(span, resp, err); err != nil {
		return nil, err
	}
	return result, nil
}

func (c *coreClient) RevokeAPIKey(ctx context.Context, keyID string) error {
	ctx, span := c.startSpan(ctx, "RevokeAPIKey", attribute.String("key.id", keyID))
	defer span.End()

	resp, err := c.request(ctx).
		Delete("/api/v1/auth/api-keys/" + keyID)

	return c.handleError(span, resp, err)
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
