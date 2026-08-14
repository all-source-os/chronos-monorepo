import { CircuitBreaker } from "./circuit-breaker";
import { type EventFolder, foldEvents } from "./fold";
import {
  type AllSourceConfig,
  AllSourceError,
  type CreatedEvent,
  type Event,
  type HealthResponse,
  type IngestEventInput,
  type PrimeProjection,
  type PrimeProjectionAck,
  type PrimeProvenance,
  type PrimeSnapshot,
  type ProjectionReplayAnalysis,
  type ProjectionReplayRun,
  type ProjectionsResponse,
  type QueryEventsParams,
  type QueryEventsResponse,
  type RetryConfig,
} from "./types";

const DEFAULT_TIMEOUT = 30_000;

const DEFAULT_RETRY: RetryConfig = {
  maxRetries: 3,
  baseDelay: 200,
  backoffFactor: 2.0,
  maxDelay: 10_000,
};

const RETRYABLE_STATUS_CODES = new Set([408, 429, 500, 502, 503, 504]);

/** Raw create-ack as returned by the gateway (id keyed as `event_id`). */
interface RawCreatedEvent {
  event_id?: string;
  id?: string;
  timestamp: string;
  version?: number;
}

function normalizeCreated(raw: RawCreatedEvent): CreatedEvent {
  return {
    id: raw.event_id ?? raw.id ?? "",
    timestamp: raw.timestamp,
    version: raw.version,
  };
}

/**
 * Strips trailing slashes in linear time.
 *
 * The previous implementation used `replace(/\/+$/, "")`. That pattern
 * backtracks polynomially: a caller-supplied `baseUrl` ending in a long run of
 * slashes (e.g. "https://x/" + "/".repeat(50_000)) makes the regex engine retry
 * every split point, hanging the thread. `baseUrl` is library input, so a
 * caller passing a value built from user data could stall their own process.
 * A scan from the end has no backtracking to do.
 */
function stripTrailingSlashes(url: string): string {
  let end = url.length;
  while (end > 0 && url.charCodeAt(end - 1) === 47 /* "/" */) end--;
  return url.slice(0, end);
}

export class AllSourceClient {
  private readonly baseUrl: string;
  private readonly apiKey: string;
  private readonly timeout: number;
  private readonly retryConfig: RetryConfig;
  private readonly circuitBreaker: CircuitBreaker;
  private readonly fetch: typeof globalThis.fetch;

  constructor(config: AllSourceConfig) {
    if (!config.baseUrl) throw new Error("baseUrl is required");
    if (!config.apiKey) throw new Error("apiKey is required");

    this.baseUrl = stripTrailingSlashes(config.baseUrl);
    this.apiKey = config.apiKey;
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;

    this.retryConfig = {
      maxRetries: config.retry?.maxRetries ?? DEFAULT_RETRY.maxRetries,
      baseDelay: config.retry?.baseDelay ?? DEFAULT_RETRY.baseDelay,
      backoffFactor: config.retry?.backoffFactor ?? DEFAULT_RETRY.backoffFactor,
      maxDelay: config.retry?.maxDelay ?? DEFAULT_RETRY.maxDelay,
    };

    this.circuitBreaker = new CircuitBreaker(config.circuitBreaker);
    this.fetch = config.fetch ?? globalThis.fetch;
  }

  /**
   * Ingest a single event into AllSource Core.
   *
   * Returns the created event's `id`, `timestamp` and `version`. The gateway
   * wraps the ack in a `{ data }` envelope and keys the id as `event_id`; this
   * unwraps and normalizes it to `id`.
   */
  async ingestEvent(event: IngestEventInput): Promise<CreatedEvent> {
    const res = await this.request<{ data: RawCreatedEvent }>("POST", "/api/v1/events", event);
    return normalizeCreated(res.data);
  }

  /**
   * Ingest a batch of events into AllSource Core.
   *
   * Returns how many were ingested and the created events
   * (`id` / `timestamp` / `version`).
   */
  async ingestBatch(
    events: IngestEventInput[]
  ): Promise<{ count: number; events: CreatedEvent[] }> {
    const res = await this.request<{ data: RawCreatedEvent[]; count: number }>(
      "POST",
      "/api/v1/events/batch",
      { events }
    );
    return { count: res.count, events: (res.data ?? []).map(normalizeCreated) };
  }

  /** Query events with optional filters. */
  async queryEvents(params: QueryEventsParams = {}): Promise<QueryEventsResponse> {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== null) {
        query.set(key, String(value));
      }
    }
    const qs = query.toString();
    const path = qs ? `/api/v1/events/query?${qs}` : "/api/v1/events/query";
    return this.request<QueryEventsResponse>("GET", path);
  }

  /** List all projections from AllSource Core. */
  async listProjections(): Promise<ProjectionsResponse> {
    return this.request<ProjectionsResponse>("GET", "/api/v1/projections");
  }

  /** Analyze replay impact without changing events or projection state. */
  async analyzeProjectionReplay(projectionName: string): Promise<ProjectionReplayAnalysis> {
    const res = await this.request<{ data: ProjectionReplayAnalysis }>(
      "POST",
      "/api/replay/preview",
      { projection_name: projectionName }
    );
    return res.data;
  }

  /** Start an atomic rebuild of one enabled tenant projection. */
  async startProjectionReplay(projectionName: string): Promise<ProjectionReplayRun> {
    const res = await this.request<{ data: ProjectionReplayRun }>("POST", "/api/replay", {
      projection_name: projectionName,
    });
    return res.data;
  }

  /** List replay runs belonging to the authenticated tenant. */
  async listProjectionReplays(): Promise<ProjectionReplayRun[]> {
    const res = await this.request<{ data: ProjectionReplayRun[] }>("GET", "/api/replay");
    return res.data;
  }

  /** Read one tenant-scoped replay run. */
  async getProjectionReplay(replayId: string): Promise<ProjectionReplayRun> {
    const res = await this.request<{ data: ProjectionReplayRun }>("GET", `/api/replay/${replayId}`);
    return res.data;
  }

  /** Cancel a running rebuild without replacing the current read-model. */
  async cancelProjectionReplay(replayId: string): Promise<ProjectionReplayRun> {
    const res = await this.request<{ data: ProjectionReplayRun }>(
      "POST",
      `/api/replay/${replayId}/cancel`,
      {}
    );
    return res.data;
  }

  /** List all Prime projection definitions from the gateway. */
  async listPrimeProjections(): Promise<PrimeProjection[]> {
    const res = await this.request<{ data: PrimeProjection[]; count: number }>(
      "GET",
      "/api/v1/prime/projections"
    );
    return res.data;
  }

  /** Define (or update) a Prime projection with per-field merge policies. */
  async definePrimeProjection(
    entityType: string,
    fieldPolicies: Record<string, string>
  ): Promise<PrimeProjectionAck> {
    const res = await this.request<{ data: PrimeProjectionAck }>(
      "POST",
      "/api/v1/prime/projections",
      { entity_type: entityType, field_policies: fieldPolicies }
    );
    return res.data;
  }

  /** Project a Prime node into a materialized snapshot. */
  async projectNode(nodeId: string): Promise<PrimeSnapshot> {
    const res = await this.request<{ data: PrimeSnapshot }>(
      "POST",
      `/api/v1/prime/nodes/${nodeId}/project`
    );
    return res.data;
  }

  /** Fetch provenance for a single field on a Prime node. Throws AllSourceError (404) when none. */
  async nodeFieldProvenance(nodeId: string, field: string): Promise<PrimeProvenance> {
    const res = await this.request<{ data: PrimeProvenance }>(
      "GET",
      `/api/v1/prime/nodes/${nodeId}/fields/${field}/provenance`
    );
    return res.data;
  }

  /** Query events and fold them into a state using the provided folder. */
  async queryAndFold<S>(params: QueryEventsParams, folder: EventFolder<S>): Promise<S | undefined> {
    const result = await this.queryEvents(params);
    return foldEvents(folder, result.events);
  }

  /** Check the health of the AllSource service. */
  async getHealth(): Promise<HealthResponse> {
    return this.request<HealthResponse>("GET", "/health");
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    this.circuitBreaker.check();

    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      "X-API-Key": this.apiKey,
      Accept: "application/json",
    };
    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    let lastError: unknown;
    const maxAttempts = this.retryConfig.maxRetries + 1;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      if (attempt > 0) {
        const delay = this.computeDelay(attempt);
        await new Promise((r) => setTimeout(r, delay));
      }

      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.timeout);

      try {
        const response = await this.fetch(url, {
          method,
          headers,
          body: body !== undefined ? JSON.stringify(body) : undefined,
          signal: controller.signal,
        });

        if (!response.ok) {
          const text = await response.text();
          let responseBody: unknown;
          try {
            responseBody = JSON.parse(text);
          } catch {
            responseBody = text;
          }
          const error = new AllSourceError(
            `AllSource API error: ${response.status} ${response.statusText}`,
            response.status,
            responseBody
          );

          if (RETRYABLE_STATUS_CODES.has(response.status) && attempt < maxAttempts - 1) {
            lastError = error;
            continue;
          }

          this.circuitBreaker.recordFailure();
          throw error;
        }

        this.circuitBreaker.recordSuccess();
        return (await response.json()) as T;
      } catch (error) {
        if (error instanceof AllSourceError) {
          if (error.isRetryable() && attempt < maxAttempts - 1) {
            lastError = error;
            continue;
          }
          this.circuitBreaker.recordFailure();
          throw error;
        }
        if (error instanceof DOMException && error.name === "AbortError") {
          const timeoutErr = new AllSourceError(`Request timeout after ${this.timeout}ms`, 0);
          if (attempt < maxAttempts - 1) {
            lastError = timeoutErr;
            continue;
          }
          this.circuitBreaker.recordFailure();
          throw timeoutErr;
        }
        // Network errors are retryable
        if (attempt < maxAttempts - 1) {
          lastError = error;
          continue;
        }
        this.circuitBreaker.recordFailure();
        throw error;
      } finally {
        clearTimeout(timer);
      }
    }

    // Should not reach here, but just in case
    this.circuitBreaker.recordFailure();
    throw lastError;
  }

  private computeDelay(attempt: number): number {
    const { baseDelay, backoffFactor, maxDelay } = this.retryConfig;
    const exponentialDelay = baseDelay * backoffFactor ** (attempt - 1);
    const capped = Math.min(exponentialDelay, maxDelay);
    // Add jitter: random value between 0 and capped delay
    const jitter = Math.random() * capped;
    return Math.floor(jitter);
  }
}
