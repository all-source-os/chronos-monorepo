import {
  type AllSourceConfig,
  AllSourceError,
  type Event,
  type HealthResponse,
  type IngestEventInput,
  type QueryEventsParams,
  type QueryEventsResponse,
} from "./types";

const DEFAULT_TIMEOUT = 30_000;

export class AllSourceClient {
  private readonly baseUrl: string;
  private readonly apiKey: string;
  private readonly timeout: number;

  constructor(config: AllSourceConfig) {
    if (!config.baseUrl) throw new Error("baseUrl is required");
    if (!config.apiKey) throw new Error("apiKey is required");

    this.baseUrl = config.baseUrl.replace(/\/+$/, "");
    this.apiKey = config.apiKey;
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
  }

  /** Ingest a single event into AllSource. */
  async ingestEvent(event: IngestEventInput): Promise<Event> {
    return this.request<Event>("POST", "/api/events", event);
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
    const path = qs ? `/api/events?${qs}` : "/api/events";
    return this.request<QueryEventsResponse>("GET", path);
  }

  /** Check the health of the AllSource service. */
  async getHealth(): Promise<HealthResponse> {
    return this.request<HealthResponse>("GET", "/api/health");
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeout);

    try {
      const headers: Record<string, string> = {
        "X-API-Key": this.apiKey,
        Accept: "application/json",
      };
      if (body !== undefined) {
        headers["Content-Type"] = "application/json";
      }

      const response = await fetch(url, {
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
        throw new AllSourceError(
          `AllSource API error: ${response.status} ${response.statusText}`,
          response.status,
          responseBody,
        );
      }

      return (await response.json()) as T;
    } catch (error) {
      if (error instanceof AllSourceError) throw error;
      if (error instanceof DOMException && error.name === "AbortError") {
        throw new AllSourceError(`Request timeout after ${this.timeout}ms`, 0);
      }
      throw error;
    } finally {
      clearTimeout(timer);
    }
  }
}
