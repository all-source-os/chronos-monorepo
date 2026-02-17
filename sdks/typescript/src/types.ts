/** Configuration options for the AllSource client. */
export interface AllSourceConfig {
  /** Base URL of the AllSource Query Service (e.g., "https://allsource-query.fly.dev"). */
  baseUrl: string;
  /** API key for authentication (sent as X-API-Key header). */
  apiKey: string;
  /** Request timeout in milliseconds. Defaults to 30000. */
  timeout?: number;
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
  start_time?: string;
  /** End time filter (ISO 8601). */
  end_time?: string;
}

/** Response from querying events. */
export interface QueryEventsResponse {
  events: Event[];
  count: number;
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
    public readonly body?: unknown,
  ) {
    super(message);
    this.name = "AllSourceError";
  }
}
