// API Client for Query Service
// All dashboard operations go through this client

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
const CONTROL_PLANE_URL = process.env.NEXT_PUBLIC_CONTROL_PLANE_URL || "http://localhost:3901";

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, string[]>;
}

export interface ApiResponse<T> {
  data?: T;
  error?: ApiError;
}

export class ApiClient {
  private baseUrl: string;
  private asOf: string | null = null;

  constructor(baseUrl: string = API_URL) {
    this.baseUrl = baseUrl;
  }

  /**
   * Set the as_of timestamp for time travel queries.
   * All subsequent requests will include this timestamp.
   * Set to null to return to present.
   */
  setAsOf(timestamp: string | null): void {
    this.asOf = timestamp;
  }

  /**
   * Get the current as_of timestamp.
   */
  getAsOf(): string | null {
    return this.asOf;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResponse<T>> {
    // Append as_of parameter if set and not already present
    let url = `${this.baseUrl}${endpoint}`;
    if (this.asOf && !endpoint.includes("as_of=")) {
      const separator = endpoint.includes("?") ? "&" : "?";
      url = `${url}${separator}as_of=${encodeURIComponent(this.asOf)}`;
    }

    const headers: HeadersInit = {
      "Content-Type": "application/json",
      ...options.headers,
    };

    try {
      const response = await fetch(url, {
        ...options,
        headers,
        credentials: "include", // Include cookies for auth
      });

      const data = await response.json();

      if (!response.ok) {
        return {
          error: data.error || {
            code: "unknown_error",
            message: response.statusText,
          },
        };
      }

      return { data: data.data ?? data };
    } catch (error) {
      return {
        error: {
          code: "network_error",
          message: error instanceof Error ? error.message : "Network error",
        },
      };
    }
  }

  // Auth endpoints
  async getMe(): Promise<ApiResponse<User>> {
    return this.request<User>("/api/auth/me");
  }

  async logout(): Promise<ApiResponse<void>> {
    return this.request<void>("/api/auth/logout", { method: "POST" });
  }

  // Tenant endpoints
  async getTenant(): Promise<ApiResponse<Tenant>> {
    return this.request<Tenant>("/api/tenant");
  }

  async updateTenant(data: UpdateTenantRequest): Promise<ApiResponse<Tenant>> {
    return this.request<Tenant>("/api/tenant", {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async getTenantUsage(): Promise<ApiResponse<TenantUsage>> {
    return this.request<TenantUsage>("/api/tenant/usage");
  }

  // Events endpoints
  async createEvent(event: CreateEventRequest): Promise<ApiResponse<Event>> {
    return this.request<Event>("/api/events", {
      method: "POST",
      body: JSON.stringify(event),
    });
  }

  async createEventBatch(events: CreateEventRequest[]): Promise<ApiResponse<Event[]>> {
    return this.request<Event[]>("/api/events/batch", {
      method: "POST",
      body: JSON.stringify({ events }),
    });
  }

  async listEvents(params?: ListEventsParams): Promise<ApiResponse<EventListResponse>> {
    const queryString = params
      ? `?${new URLSearchParams(
          Object.entries(params).reduce(
            (acc, [key, value]) => {
              if (value !== undefined) acc[key] = String(value);
              return acc;
            },
            {} as Record<string, string>
          )
        ).toString()}`
      : "";
    return this.request<EventListResponse>(`/api/events${queryString}`);
  }

  async getEventsByEntity(entityId: string): Promise<ApiResponse<EventListResponse>> {
    return this.request<EventListResponse>(`/api/events/entity/${encodeURIComponent(entityId)}`);
  }

  async getEventsByType(eventType: string): Promise<ApiResponse<EventListResponse>> {
    return this.request<EventListResponse>(`/api/events/type/${encodeURIComponent(eventType)}`);
  }

  // Streams (entity discovery) endpoint
  async listStreams(params?: {
    limit?: number;
    offset?: number;
  }): Promise<ApiResponse<StreamsListResponse>> {
    const queryString = params
      ? `?${new URLSearchParams(
          Object.entries(params).reduce(
            (acc, [key, value]) => {
              if (value !== undefined) acc[key] = String(value);
              return acc;
            },
            {} as Record<string, string>
          )
        ).toString()}`
      : "";
    return this.request<StreamsListResponse>(`/api/streams${queryString}`);
  }

  // Event types discovery endpoint
  async listEventTypes(params?: {
    limit?: number;
    offset?: number;
  }): Promise<ApiResponse<EventTypesListResponse>> {
    const queryString = params
      ? `?${new URLSearchParams(
          Object.entries(params).reduce(
            (acc, [key, value]) => {
              if (value !== undefined) acc[key] = String(value);
              return acc;
            },
            {} as Record<string, string>
          )
        ).toString()}`
      : "";
    return this.request<EventTypesListResponse>(`/api/event-types${queryString}`);
  }

  // Query endpoint
  async executeQuery(query: QueryRequest): Promise<ApiResponse<QueryResponse>> {
    return this.request<QueryResponse>("/api/query", {
      method: "POST",
      body: JSON.stringify(query),
    });
  }

  // API Keys endpoints
  async listApiKeys(): Promise<ApiResponse<ApiKey[]>> {
    return this.request<ApiKey[]>("/api/api-keys");
  }

  async createApiKey(data: CreateApiKeyRequest): Promise<ApiResponse<ApiKeyWithSecret>> {
    return this.request<ApiKeyWithSecret>("/api/api-keys", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async getApiKey(id: string): Promise<ApiResponse<ApiKey>> {
    return this.request<ApiKey>(`/api/api-keys/${id}`);
  }

  async updateApiKey(id: string, data: UpdateApiKeyRequest): Promise<ApiResponse<ApiKey>> {
    return this.request<ApiKey>(`/api/api-keys/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async rotateApiKey(id: string): Promise<ApiResponse<ApiKeyWithSecret>> {
    return this.request<ApiKeyWithSecret>(`/api/api-keys/${id}/rotate`, {
      method: "POST",
    });
  }

  async revokeApiKey(id: string): Promise<ApiResponse<void>> {
    return this.request<void>(`/api/api-keys/${id}`, {
      method: "DELETE",
    });
  }

  async getApiKeyScopes(): Promise<ApiResponse<string[]>> {
    return this.request<string[]>("/api/api-keys/scopes");
  }

  // Billing endpoints
  async createCheckout(
    variantId: string,
    billingPeriod: "monthly" | "annual" = "monthly"
  ): Promise<ApiResponse<CheckoutResponse>> {
    return this.request<CheckoutResponse>("/api/billing/checkout", {
      method: "POST",
      body: JSON.stringify({ variant_id: variantId, billing_period: billingPeriod }),
    });
  }

  async getBillingPortal(): Promise<ApiResponse<BillingPortalResponse>> {
    return this.request<BillingPortalResponse>("/api/billing/portal");
  }

  async getOverage(): Promise<ApiResponse<OverageResponse>> {
    return this.request<OverageResponse>("/api/billing/overage");
  }

  async enableOverage(rates?: OverageRates): Promise<ApiResponse<void>> {
    return this.request<void>("/api/billing/overage/enable", {
      method: "POST",
      body: JSON.stringify(rates || {}),
    });
  }

  async disableOverage(): Promise<ApiResponse<void>> {
    return this.request<void>("/api/billing/overage/disable", {
      method: "POST",
    });
  }

  async getProjectedCharges(): Promise<ApiResponse<ProjectedChargesResponse>> {
    return this.request<ProjectedChargesResponse>("/api/billing/projected-charges");
  }

  // Projections endpoints
  async listProjections(): Promise<ApiResponse<Projection[]>> {
    return this.request<Projection[]>("/api/projections");
  }

  async getProjection(name: string): Promise<ApiResponse<Projection>> {
    return this.request<Projection>(`/api/projections/${encodeURIComponent(name)}`);
  }

  async createProjection(data: CreateProjectionRequest): Promise<ApiResponse<Projection>> {
    return this.request<Projection>("/api/projections", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  // Metrics endpoints
  async getMetrics(): Promise<ApiResponse<MetricsResponse>> {
    return this.request<MetricsResponse>("/api/metrics");
  }

  // Analytics endpoints
  async getAnalyticsStats(params?: { as_of?: string }): Promise<ApiResponse<AnalyticsStats>> {
    const queryString = params?.as_of ? `?as_of=${encodeURIComponent(params.as_of)}` : "";
    return this.request<AnalyticsStats>(`/api/analytics/stats${queryString}`);
  }

  // Events replay for time range
  async getEventsInRange(params: {
    start: string;
    end: string;
    entity_id?: string;
    event_type?: string;
    limit?: number;
  }): Promise<ApiResponse<EventListResponse>> {
    const queryParams = new URLSearchParams();
    queryParams.set("start", params.start);
    queryParams.set("end", params.end);
    if (params.entity_id) queryParams.set("entity_id", params.entity_id);
    if (params.event_type) queryParams.set("event_type", params.event_type);
    if (params.limit) queryParams.set("limit", String(params.limit));

    return this.request<EventListResponse>(`/api/events/range?${queryParams.toString()}`);
  }

  // Team management endpoints
  async listTeamMembers(): Promise<ApiResponse<TeamMembersResponse>> {
    return this.request<TeamMembersResponse>("/api/team/members");
  }

  async inviteTeamMember(data: InviteMemberRequest): Promise<ApiResponse<Invitation>> {
    return this.request<Invitation>("/api/team/invite", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async removeTeamMember(userId: string): Promise<ApiResponse<void>> {
    return this.request<void>(`/api/team/members/${userId}`, {
      method: "DELETE",
    });
  }

  async updateTeamMemberRole(userId: string, role: string): Promise<ApiResponse<TeamMember>> {
    return this.request<TeamMember>(`/api/team/members/${userId}/role`, {
      method: "PUT",
      body: JSON.stringify({ role }),
    });
  }

  // Audit log endpoints
  async listAuditLogs(params?: AuditLogParams): Promise<ApiResponse<AuditLogResponse>> {
    const queryString = params
      ? `?${new URLSearchParams(
          Object.entries(params).reduce(
            (acc, [key, value]) => {
              if (value !== undefined) acc[key] = String(value);
              return acc;
            },
            {} as Record<string, string>
          )
        ).toString()}`
      : "";
    return this.request<AuditLogResponse>(`/api/tenant/audit-logs${queryString}`);
  }

  // Usage analytics endpoints
  async getUsageAnalytics(params?: {
    range?: string;
  }): Promise<ApiResponse<UsageAnalyticsResponse>> {
    const queryString = params?.range ? `?range=${encodeURIComponent(params.range)}` : "";
    return this.request<UsageAnalyticsResponse>(`/api/tenants/me/analytics${queryString}`);
  }

  // Replay endpoints
  async startReplay(params: StartReplayRequest): Promise<ApiResponse<ReplayProgress>> {
    return this.request<ReplayProgress>("/api/replay", {
      method: "POST",
      body: JSON.stringify(params),
    });
  }

  async listReplays(): Promise<ApiResponse<ReplayListResponse>> {
    return this.request<ReplayListResponse>("/api/replay");
  }

  async getReplay(replayId: string): Promise<ApiResponse<ReplayProgress>> {
    return this.request<ReplayProgress>(`/api/replay/${replayId}`);
  }

  async cancelReplay(replayId: string): Promise<ApiResponse<ReplayProgress>> {
    return this.request<ReplayProgress>(`/api/replay/${replayId}/cancel`, {
      method: "POST",
    });
  }

  async deleteReplay(replayId: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return this.request<{ deleted: boolean }>(`/api/replay/${replayId}`, {
      method: "DELETE",
    });
  }

  // Entity timeline (formatted for visualization)
  async getEntityTimeline(
    entityId: string,
    params?: { as_of?: string }
  ): Promise<ApiResponse<EntityTimeline>> {
    const queryString = params?.as_of ? `?as_of=${encodeURIComponent(params.as_of)}` : "";
    return this.request<EntityTimeline>(
      `/api/entities/${encodeURIComponent(entityId)}/timeline${queryString}`
    );
  }
}

// Types
export interface User {
  id: string;
  email: string;
  name: string;
  avatar_url: string | null;
  provider: "google" | "github" | "email";
  email_verified: boolean;
  tenant_id: string;
}

export interface Tenant {
  id: string;
  name: string;
  slug: string;
  subscription_status: "active" | "trialing" | "past_due" | "cancelled" | "expired";
  subscription_tier: "free" | "starter" | "growth" | "enterprise";
  billing_period: "monthly" | "annual" | null;
  trial_ends_at: string | null;
  subscription_ends_at: string | null;
  events_quota: number;
  queries_quota: number;
  events_used: number;
  queries_used: number;
  settings: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface UpdateTenantRequest {
  name?: string;
  settings?: Record<string, unknown>;
}

export interface TenantUsage {
  tenant_id: string;
  subscription_tier: string;
  subscription_status: string;
  events: {
    used: number;
    quota: number;
    percentage: number;
  };
  queries: {
    used: number;
    quota: number;
    percentage: number;
  };
  billing_period: {
    reset_at: string;
  };
}

export interface Event {
  id: string;
  entity_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  timestamp: string;
  version: number;
}

export interface CreateEventRequest {
  entity_id: string;
  event_type: string;
  payload: Record<string, unknown>;
}

export interface ListEventsParams {
  entity_id?: string;
  event_type?: string;
  limit?: number;
  offset?: number;
  as_of?: string;
}

export interface EventListResponse {
  data: Event[];
  count: number;
}

export interface StreamInfo {
  stream_id: string;
  event_count: number;
  last_event_at: string | null;
}

export interface StreamsListResponse {
  data: StreamInfo[];
  count: number;
  total: number;
}

export interface EventTypeInfo {
  event_type: string;
  event_count: number;
  last_event_at: string | null;
}

export interface EventTypesListResponse {
  data: EventTypeInfo[];
  count: number;
  total: number;
}

export interface QueryRequest {
  entity_id?: string;
  event_type?: string;
  from?: string;
  where?: QueryCondition;
  limit?: number;
  offset?: number;
}

export interface QueryCondition {
  op?: string;
  field?: string;
  value?: unknown;
  and?: QueryCondition[];
  or?: QueryCondition[];
}

export interface QueryResponse {
  data: Event[];
  count: number;
  query: {
    from: string;
    has_filter: boolean;
    limit: number;
    offset: number;
    type: string;
  };
}

export interface ApiKey {
  id: string;
  name: string;
  description: string | null;
  key_prefix: string;
  scopes: string[];
  last_used_at: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface ApiKeyWithSecret extends ApiKey {
  key: string;
}

export interface CreateApiKeyRequest {
  name: string;
  description?: string;
  scopes: string[];
  expires_at?: string;
}

export interface UpdateApiKeyRequest {
  name?: string;
  description?: string;
  scopes?: string[];
}

export interface CheckoutResponse {
  checkout_url: string;
}

export interface BillingPortalResponse {
  portal_url?: string;
  has_subscription: boolean;
  message?: string;
}

export interface OverageResponse {
  overage_enabled: boolean;
  events: {
    used: number;
    quota: number;
    overage: number;
    rate_cents: number;
  };
  queries: {
    used: number;
    quota: number;
    overage: number;
    rate_cents: number;
  };
  estimated_charges_cents: number;
}

export interface OverageRates {
  events_rate?: number;
  queries_rate?: number;
}

export interface ProjectedChargesResponse {
  subscription_tier: string;
  overage_enabled: boolean;
  usage: TenantUsage;
  overage_charges: {
    events_overage: number;
    queries_overage: number;
    events_charges_cents: number;
    queries_charges_cents: number;
    total_charges_cents: number;
  };
  billing_period: {
    reset_at: string;
  };
}

export interface Projection {
  id: string;
  name: string;
  version: number;
  status: "running" | "paused" | "error";
  initial_state: Record<string, unknown>;
  definition: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProjectionRequest {
  name: string;
  version: number;
  initial_state: Record<string, unknown>;
  definition: string;
}

export interface MetricsResponse {
  service: string;
  timestamp: string;
  elixir: {
    processes: number;
    memory: Record<string, number>;
    uptime_seconds: number;
    schedulers: number;
  };
  backend: Record<string, unknown>;
}

export interface AnalyticsStats {
  events: {
    total: number;
    by_type: Record<string, number>;
    recent_rate: number; // events per minute
  };
  entities: {
    total: number;
    active: number;
  };
  errors: {
    count: number;
    rate: number;
  };
  latency: {
    p50_us: number;
    p99_us: number;
  };
  as_of?: string;
}

export interface EntityTimeline {
  entity_id: string;
  events: TimelineEvent[];
  gaps: TimelineGap[];
  as_of?: string;
}

export interface TimelineEvent {
  id: string;
  event_type: string;
  timestamp: string;
  payload: Record<string, unknown>;
  duration_since_previous_ms?: number;
}

export interface TimelineGap {
  start: string;
  end: string;
  duration_ms: number;
  expected_event_types?: string[];
}

// Team Management types
export interface TeamMember {
  id: string;
  user_id: string;
  email: string;
  name: string;
  role: "owner" | "admin" | "member" | "viewer";
  joined_at: string;
  status: "active" | "pending";
}

export interface TeamMembersResponse {
  members: TeamMember[];
  seat_limit: number;
  seats_used: number;
}

export interface InviteMemberRequest {
  email: string;
  role: "admin" | "member" | "viewer";
}

export interface Invitation {
  id: string;
  email: string;
  role: string;
  invited_by: string;
  invited_at: string;
  status: "pending" | "accepted" | "expired";
}

// Audit Log types
export interface AuditLogEntry {
  id: string;
  timestamp: string;
  actor: string;
  action: string;
  details: Record<string, unknown>;
}

export interface AuditLogResponse {
  entries: AuditLogEntry[];
  retention_days: number;
  actions: string[];
}

export interface AuditLogParams {
  action?: string;
  limit?: number;
  offset?: number;
}

// Replay types
export type ReplayStatus = "Pending" | "Running" | "Completed" | "Failed" | "Cancelled";

export interface ReplayConfig {
  batch_size?: number;
  parallel?: boolean;
  workers?: number;
  emit_progress?: boolean;
  progress_interval?: number;
}

export interface StartReplayRequest {
  from_timestamp?: string;
  to_timestamp?: string;
  event_type?: string;
  entity_id?: string;
  projection_name?: string;
  config?: ReplayConfig;
}

export interface ReplayProgress {
  replay_id: string;
  status: ReplayStatus;
  started_at: string;
  updated_at: string;
  completed_at: string | null;
  total_events: number;
  processed_events: number;
  failed_events: number;
  progress_percentage: number;
  events_per_second: number;
  error_message: string | null;
}

export interface ReplayListResponse {
  data: ReplayProgress[];
  total: number;
}

// Usage Analytics types
export interface EventTypeDistribution {
  event_type: string;
  count: number;
}

export interface TopEntity {
  entity_id: string;
  event_count: number;
}

export interface IngestionDataPoint {
  timestamp: string;
  count: number;
}

export interface UsageAnalyticsResponse {
  range: string;
  since: string;
  event_type_distribution: EventTypeDistribution[];
  top_entity_ids: TopEntity[];
  ingestion_rate: IngestionDataPoint[];
}

// Export singleton instance
export const apiClient = new ApiClient();

// Export API URL for Query Service requests
export const getApiUrl = () => API_URL;

// Export Control Plane URL for auth requests (login, register, OAuth)
export const getControlPlaneUrl = () => CONTROL_PLANE_URL;
