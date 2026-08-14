/** Retry configuration with exponential backoff. */
export interface RetryConfig {
  /** Maximum number of retries. Defaults to 3. */
  maxRetries: number;
  /** Base delay in milliseconds. Defaults to 200. */
  baseDelay: number;
  /** Backoff multiplier. Defaults to 2.0. */
  backoffFactor: number;
  /** Maximum delay in milliseconds. Defaults to 10000. */
  maxDelay: number;
}

/** Circuit breaker configuration. */
export interface CircuitBreakerConfig {
  /** Number of consecutive failures before opening the circuit. Defaults to 5. */
  threshold: number;
  /** Milliseconds to wait before entering half-open state. Defaults to 30000. */
  recoveryTimeout: number;
}

/** Configuration options for the AllSource client. */
export interface AllSourceConfig {
  /** Base URL of the AllSource Query Service (e.g., "https://allsource-query.fly.dev"). */
  baseUrl: string;
  /** API key for authentication (sent as X-API-Key header). */
  apiKey: string;
  /** Request timeout in milliseconds. Defaults to 30000. */
  timeout?: number;
  /** Retry configuration. Uses sensible defaults if omitted. */
  retry?: Partial<RetryConfig>;
  /** Circuit breaker configuration. Uses sensible defaults if omitted. */
  circuitBreaker?: Partial<CircuitBreakerConfig>;
  /** Custom fetch function. Defaults to globalThis.fetch. */
  fetch?: typeof globalThis.fetch;
}

/** An event to ingest into AllSource. */
export interface IngestEventInput {
  /** The type of event (e.g., "user.signup", "order.placed"). */
  event_type: string;
  /** The entity this event belongs to (e.g., user ID, order ID). */
  entity_id: string;
  /** Arbitrary JSON payload for the event. */
  payload: Record<string, unknown>;
  /** Optional metadata (e.g., source, version, ip). */
  metadata?: Record<string, unknown>;
}

/** A stored event returned from AllSource. */
export interface Event {
  id: string;
  event_type: string;
  entity_id: string;
  payload: Record<string, unknown>;
  metadata: Record<string, unknown>;
  timestamp: string;
  stream_id?: string;
  version?: number;
  tenant_id?: string;
}

/**
 * Acknowledgement for a created event, returned by `ingestEvent` and (per item)
 * `ingestBatch`. The gateway keys the id as `event_id`; the SDK normalizes it
 * to `id` for parity with queried events.
 */
export interface CreatedEvent {
  /** The stored event's id. */
  id: string;
  /** Server-assigned timestamp (ISO 8601). */
  timestamp: string;
  /** Monotonic version assigned by Core, when provided. */
  version?: number;
}

/** Query parameters for filtering events. */
export interface QueryEventsParams {
  /** Filter by entity ID. */
  entity_id?: string;
  /** Filter by event type. */
  event_type?: string;
  /** Maximum number of events to return. */
  limit?: number;
  /** Number of events to skip. */
  offset?: number;
  /** Start time filter (ISO 8601). */
  since?: string;
  /** End time filter (ISO 8601). */
  until?: string;
}

/** Response from querying events. */
export interface QueryEventsResponse {
  events: Event[];
  count: number;
}

/** A projection returned from AllSource Core. */
export interface Projection {
  name: string;
  state?: unknown;
  [key: string]: unknown;
}

/** Response from listing projections. */
export interface ProjectionsResponse {
  projections: Projection[];
  total: number;
}

/** One event-type bucket from a replay impact analysis. */
export interface ProjectionReplayEventType {
  event_type: string;
  count: number;
  share: number;
}

/** One frequently affected entity from a replay impact analysis sample. */
export interface ProjectionReplayEntity {
  entity_id: string;
  event_count: number;
}

/** Server-asserted invariant checked before a projection replay. */
export interface ProjectionReplayCheck {
  key: string;
  label: string;
  status: "pass" | "warn";
  detail: string;
}

/** Read-only impact analysis returned before a projection replay starts. */
export interface ProjectionReplayAnalysis {
  projection_name: string;
  projection_title: string;
  projection_kind: string;
  projection_status: string;
  current_entity_count: number;
  total_events: number;
  sampled_events: number;
  analysis_scope: "full" | "sample";
  event_type_distribution: ProjectionReplayEventType[];
  sampled_entity_count: number;
  sampled_entities: ProjectionReplayEntity[];
  first_event_at: string | null;
  last_event_at: string | null;
  analyzed_at: string;
  ready_to_replay: boolean;
  checks: ProjectionReplayCheck[];
  warnings: string[];
}

export type ProjectionReplayStatus = "pending" | "running" | "completed" | "failed" | "cancelled";

/** Tenant-scoped projection replay run. */
export interface ProjectionReplayRun {
  replay_id: string;
  projection_name: string;
  status: ProjectionReplayStatus;
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

/** A Prime projection definition (entity type plus per-field merge policies). */
export interface PrimeProjection {
  entity_type: string;
  field_policies: Record<string, string>;
}

/** Acknowledgement returned when defining a Prime projection. */
export interface PrimeProjectionAck {
  entity_type: string;
  persisted: boolean;
}

/** A materialized Prime node snapshot. */
export interface PrimeSnapshot {
  entity_type: string;
  fields: Record<string, unknown>;
  observation_count: number;
}

/** Provenance for a single field on a Prime node. */
export interface PrimeProvenance {
  field: string;
  value: unknown;
  source_event_id: string;
  source_event_at: string;
  merge_policy_applied: string;
}

/** Response from the health endpoint. */
export interface HealthResponse {
  status: string;
  [key: string]: unknown;
}

/** Error thrown by the AllSource client. */
export class AllSourceError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly body?: unknown
  ) {
    super(message);
    this.name = "AllSourceError";
  }

  /** Whether the error is a 401 Unauthorized. */
  isUnauthorized(): boolean {
    return this.status === 401;
  }

  /** Whether the error is a 429 Rate Limited. */
  isRateLimited(): boolean {
    return this.status === 429;
  }

  /** Whether the error is a 404 Not Found. */
  isNotFound(): boolean {
    return this.status === 404;
  }

  /** Whether the error is a 403 Forbidden. */
  isForbidden(): boolean {
    return this.status === 403;
  }

  /** Whether the error is a server error (5xx). */
  isServerError(): boolean {
    return this.status >= 500;
  }

  /** Whether the error is retryable (408, 429, 500, 502, 503, 504). */
  isRetryable(): boolean {
    return [408, 429, 500, 502, 503, 504].includes(this.status);
  }
}

/** Error thrown when the circuit breaker is open. */
export class CircuitOpenError extends Error {
  constructor(message = "Circuit breaker is open") {
    super(message);
    this.name = "CircuitOpenError";
  }
}
