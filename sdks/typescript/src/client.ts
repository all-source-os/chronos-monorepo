import { CircuitBreaker } from "./circuit-breaker";
import { type EventFolder, foldEvents } from "./fold";
import {
  type AllSourceConfig,
  AllSourceError,
  type Event,
  type HealthResponse,
  type IngestEventInput,
  type PrimeProjection,
  type PrimeProjectionAck,
  type PrimeProvenance,
  type PrimeSnapshot,
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

    this.baseUrl = config.baseUrl.replace(/\/+$/, "");
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

  /** Ingest a single event into AllSource Core. */
  async ingestEvent(
    event: IngestEventInput,
  ): Promise<{ event_id: string; timestamp: string }> {
    return this.request("POST", "/api/v1/events", event);
  }

  /** Ingest a batch of events into AllSource Core. */
  async ingestBatch(
    events: IngestEventInput[],
  ): Promise<{
    total: number;
    ingested: number;
    events: Array<{ event_id: string; timestamp: string }>;
  }> {
    return this.request("POST", "/api/v1/events/batch", { events });
  }

  /** Query events with optional filters. */
  async queryEvents(
    params: QueryEventsParams = {},
  ): Promise<QueryEventsResponse> {
    const query = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined && value !== null) {
        query.set(key, String(value));
      }
    }
    const qs = query.toString();
    const path = qs
      ? `/api/v1/events/query?${qs}`
      : "/api/v1/events/query";
    return this.request<QueryEventsResponse>("GET", path);
  }

  /** List all projections from AllSource Core. */
  async listProjections(): Promise<ProjectionsResponse> {
    return this.request<ProjectionsResponse>("GET", "/api/v1/projections");
  }

  /** List all Prime projection definitions from the gateway. */
  async listPrimeProjections(): Promise<PrimeProjection[]> {
    const res = await this.request<{ data: PrimeProjection[]; count: number }>(
      "GET",
      "/api/v1/prime/projections",
    );
    return res.data;
  }

  /** Define (or update) a Prime projection with per-field merge policies. */
  async definePrimeProjection(
    entityType: string,
    fieldPolicies: Record<string, string>,
  ): Promise<PrimeProjectionAck> {
    const res = await this.request<{ data: PrimeProjectionAck }>(
      "POST",
      "/api/v1/prime/projections",
      { entity_type: entityType, field_policies: fieldPolicies },
    );
    return res.data;
  }

  /** Project a Prime node into a materialized snapshot. */
  async projectNode(nodeId: string): Promise<PrimeSnapshot> {
    const res = await this.request<{ data: PrimeSnapshot }>(
      "POST",
      `/api/v1/prime/nodes/${nodeId}/project`,
    );
    return res.data;
  }

  /** Fetch provenance for a single field on a Prime node. Throws AllSourceError (404) when none. */
  async nodeFieldProvenance(
    nodeId: string,
    field: string,
  ): Promise<PrimeProvenance> {
    const res = await this.request<{ data: PrimeProvenance }>(
      "GET",
      `/api/v1/prime/nodes/${nodeId}/fields/${field}/provenance`,
    );
    return res.data;
  }

  /** Query events and fold them into a state using the provided folder. */
  async queryAndFold<S>(
    params: QueryEventsParams,
    folder: EventFolder<S>,
  ): Promise<S | undefined> {
    const result = await this.queryEvents(params);
    return foldEvents(folder, result.events);
  }

  /** Check the health of the AllSource service. */
  async getHealth(): Promise<HealthResponse> {
    return this.request<HealthResponse>("GET", "/health");
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
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
            responseBody,
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
          const timeoutErr = new AllSourceError(
            `Request timeout after ${this.timeout}ms`,
            0,
          );
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
    const exponentialDelay = baseDelay * Math.pow(backoffFactor, attempt - 1);
    const capped = Math.min(exponentialDelay, maxDelay);
    // Add jitter: random value between 0 and capped delay
    const jitter = Math.random() * capped;
    return Math.floor(jitter);
  }
}
