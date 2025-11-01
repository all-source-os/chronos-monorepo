import type {
  CreateEventRequest,
  Event,
  QueryEventsParams,
  Projection,
  Schema,
  Snapshot,
  Pipeline,
  AnalyticsQuery,
  Tenant,
  Metrics,
} from "./types";

const EVENT_STORE_URL = process.env.NEXT_PUBLIC_EVENT_STORE_URL || "http://localhost:3900";
const CONTROL_PLANE_URL = process.env.NEXT_PUBLIC_CONTROL_PLANE_URL || "http://localhost:3901";

class EventStoreClient {
  private baseUrl: string;
  private controlPlaneUrl: string;
  private apiKey?: string;
  private token?: string;

  constructor(baseUrl: string = EVENT_STORE_URL, controlPlaneUrl: string = CONTROL_PLANE_URL) {
    this.baseUrl = baseUrl;
    this.controlPlaneUrl = controlPlaneUrl;
    // Try to get API key from environment
    this.apiKey = process.env.NEXT_PUBLIC_EVENT_STORE_API_KEY;
  }

  /**
   * Set API key for authentication
   */
  setApiKey(apiKey: string): void {
    this.apiKey = apiKey;
  }

  /**
   * Set JWT token for authentication
   */
  setToken(token: string): void {
    this.token = token;
  }

  /**
   * Get headers with authentication
   */
  private getHeaders(includeContentType = true): HeadersInit {
    const headers: HeadersInit = {};

    if (includeContentType) {
      headers["Content-Type"] = "application/json";
    }

    // Add authentication header if available
    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    } else if (this.apiKey) {
      headers["X-API-Key"] = this.apiKey;
    }

    return headers;
  }

  // Event Management
  async createEvent(event: CreateEventRequest): Promise<Event> {
    const response = await fetch(`${this.baseUrl}/api/events`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(event),
    });
    if (!response.ok) throw new Error(`Failed to create event: ${response.statusText}`);
    return response.json();
  }

  async createEventBatch(events: CreateEventRequest[]): Promise<Event[]> {
    const response = await fetch(`${this.baseUrl}/api/events/batch`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify({ events }),
    });
    if (!response.ok) throw new Error(`Failed to create events: ${response.statusText}`);
    return response.json();
  }

  async queryEvents(params: QueryEventsParams): Promise<Event[]> {
    const queryString = new URLSearchParams(
      Object.entries(params).reduce(
        (acc, [key, value]) => {
          if (value !== undefined) acc[key] = String(value);
          return acc;
        },
        {} as Record<string, string>
      )
    ).toString();

    const response = await fetch(`${this.baseUrl}/api/events?${queryString}`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to query events: ${response.statusText}`);
    return response.json();
  }

  async getEventsByEntity(entityId: string): Promise<Event[]> {
    return this.queryEvents({ entity_id: entityId });
  }

  async getEventsByType(eventType: string): Promise<Event[]> {
    return this.queryEvents({ event_type: eventType });
  }

  // Projections
  async listProjections(): Promise<Projection[]> {
    const response = await fetch(`${this.baseUrl}/api/projections`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to list projections: ${response.statusText}`);
    return response.json();
  }

  async getProjection(id: string): Promise<Projection> {
    const response = await fetch(`${this.baseUrl}/api/projections/${id}`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get projection: ${response.statusText}`);
    return response.json();
  }

  async createProjection(
    projection: Omit<Projection, "id" | "created_at" | "updated_at">
  ): Promise<Projection> {
    const response = await fetch(`${this.baseUrl}/api/projections`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(projection),
    });
    if (!response.ok) throw new Error(`Failed to create projection: ${response.statusText}`);
    return response.json();
  }

  async pauseProjection(id: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/projections/${id}/pause`, {
      method: "POST",
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to pause projection: ${response.statusText}`);
  }

  async resumeProjection(id: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/api/projections/${id}/resume`, {
      method: "POST",
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to resume projection: ${response.statusText}`);
  }

  // Schemas
  async listSchemas(): Promise<Schema[]> {
    const response = await fetch(`${this.baseUrl}/api/schemas`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to list schemas: ${response.statusText}`);
    return response.json();
  }

  async getSchema(eventType: string, version?: number): Promise<Schema> {
    const url = version
      ? `${this.baseUrl}/api/schemas/${eventType}?version=${version}`
      : `${this.baseUrl}/api/schemas/${eventType}`;
    const response = await fetch(url, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get schema: ${response.statusText}`);
    return response.json();
  }

  async registerSchema(schema: Omit<Schema, "id" | "created_at">): Promise<Schema> {
    const response = await fetch(`${this.baseUrl}/api/schemas`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(schema),
    });
    if (!response.ok) throw new Error(`Failed to register schema: ${response.statusText}`);
    return response.json();
  }

  // Snapshots
  async listSnapshots(entityId?: string): Promise<Snapshot[]> {
    const url = entityId
      ? `${this.baseUrl}/api/snapshots?entity_id=${entityId}`
      : `${this.baseUrl}/api/snapshots`;
    const response = await fetch(url, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to list snapshots: ${response.statusText}`);
    return response.json();
  }

  async createSnapshot(entityId: string, snapshotType: string): Promise<Snapshot> {
    const response = await fetch(`${this.baseUrl}/api/snapshots`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify({ entity_id: entityId, snapshot_type: snapshotType }),
    });
    if (!response.ok) throw new Error(`Failed to create snapshot: ${response.statusText}`);
    return response.json();
  }

  // Pipelines
  async listPipelines(): Promise<Pipeline[]> {
    const response = await fetch(`${this.baseUrl}/api/pipelines`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to list pipelines: ${response.statusText}`);
    return response.json();
  }

  async createPipeline(pipeline: Omit<Pipeline, "id" | "created_at">): Promise<Pipeline> {
    const response = await fetch(`${this.baseUrl}/api/pipelines`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(pipeline),
    });
    if (!response.ok) throw new Error(`Failed to create pipeline: ${response.statusText}`);
    return response.json();
  }

  // Analytics
  async runAnalytics(query: AnalyticsQuery): Promise<unknown> {
    const response = await fetch(`${this.baseUrl}/api/analytics`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify(query),
    });
    if (!response.ok) throw new Error(`Failed to run analytics: ${response.statusText}`);
    return response.json();
  }

  // Tenants
  async listTenants(): Promise<Tenant[]> {
    const response = await fetch(`${this.baseUrl}/api/tenants`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to list tenants: ${response.statusText}`);
    return response.json();
  }

  async getTenant(id: string): Promise<Tenant> {
    const response = await fetch(`${this.baseUrl}/api/tenants/${id}`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get tenant: ${response.statusText}`);
    return response.json();
  }

  // Metrics & Monitoring
  async getMetrics(): Promise<Metrics> {
    const response = await fetch(`${this.baseUrl}/api/metrics`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get metrics: ${response.statusText}`);
    return response.json();
  }

  async getHealth(): Promise<{ status: string; timestamp: string }> {
    const response = await fetch(`${this.baseUrl}/health`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get health: ${response.statusText}`);
    return response.json();
  }

  async getControlPlaneHealth(): Promise<{ status: string; timestamp: string }> {
    const response = await fetch(`${this.controlPlaneUrl}/health`, {
      headers: this.getHeaders(false),
    });
    if (!response.ok) throw new Error(`Failed to get control plane health: ${response.statusText}`);
    return response.json();
  }

  // WebSocket for real-time streaming
  createEventStream(params: QueryEventsParams): WebSocket {
    const queryString = new URLSearchParams(
      Object.entries(params).reduce(
        (acc, [key, value]) => {
          if (value !== undefined) acc[key] = String(value);
          return acc;
        },
        {} as Record<string, string>
      )
    ).toString();

    const wsUrl = this.baseUrl.replace("http://", "ws://").replace("https://", "wss://");
    return new WebSocket(`${wsUrl}/api/events/stream?${queryString}`);
  }
}

export const eventStoreClient = new EventStoreClient();
