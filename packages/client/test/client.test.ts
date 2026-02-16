import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { AllSourceClient, AllSourceError } from "../src";

const MOCK_BASE_URL = "https://allsource-query.example.com";
const MOCK_API_KEY = "test-api-key-123";

function createClient(overrides?: { baseUrl?: string; apiKey?: string; timeout?: number }) {
  return new AllSourceClient({
    baseUrl: overrides?.baseUrl ?? MOCK_BASE_URL,
    apiKey: overrides?.apiKey ?? MOCK_API_KEY,
    ...overrides,
  });
}

// Mock fetch globally
const originalFetch = globalThis.fetch;
let mockFetch: ReturnType<typeof mock>;

beforeEach(() => {
  mockFetch = mock(() =>
    Promise.resolve(new Response(JSON.stringify({}), { status: 200 })),
  );
  globalThis.fetch = mockFetch as unknown as typeof fetch;
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("AllSourceClient constructor", () => {
  test("throws if baseUrl is empty", () => {
    expect(() => createClient({ baseUrl: "" })).toThrow("baseUrl is required");
  });

  test("throws if apiKey is empty", () => {
    expect(() => createClient({ apiKey: "" })).toThrow("apiKey is required");
  });

  test("strips trailing slashes from baseUrl", async () => {
    const client = createClient({ baseUrl: "https://example.com///" });
    mockFetch.mockImplementation(() =>
      Promise.resolve(new Response(JSON.stringify({ status: "ok" }), { status: 200 })),
    );
    await client.getHealth();
    const url = (mockFetch.mock.calls[0] as unknown[])[0] as string;
    expect(url).toBe("https://example.com/api/health");
  });
});

describe("getHealth", () => {
  test("sends GET /api/health with API key header", async () => {
    const client = createClient();
    mockFetch.mockImplementation(() =>
      Promise.resolve(
        new Response(JSON.stringify({ status: "ok" }), { status: 200 }),
      ),
    );

    const result = await client.getHealth();

    expect(result).toEqual({ status: "ok" });
    expect(mockFetch).toHaveBeenCalledTimes(1);

    const [url, options] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(`${MOCK_BASE_URL}/api/health`);
    expect(options.method).toBe("GET");
    expect((options.headers as Record<string, string>)["X-API-Key"]).toBe(MOCK_API_KEY);
  });
});

describe("ingestEvent", () => {
  test("sends POST /api/events with event body", async () => {
    const client = createClient();
    const event = {
      event_type: "user.signup",
      entity_id: "user-123",
      payload: { email: "test@example.com" },
      metadata: { source: "sdk-test" },
    };
    const responseEvent = { id: "evt-1", ...event, timestamp: "2026-02-16T00:00:00Z" };

    mockFetch.mockImplementation(() =>
      Promise.resolve(new Response(JSON.stringify(responseEvent), { status: 200 })),
    );

    const result = await client.ingestEvent(event);

    expect(result.id).toBe("evt-1");
    expect(result.event_type).toBe("user.signup");

    const [url, options] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(url).toBe(`${MOCK_BASE_URL}/api/events`);
    expect(options.method).toBe("POST");
    expect((options.headers as Record<string, string>)["Content-Type"]).toBe("application/json");
    expect(JSON.parse(options.body as string)).toEqual(event);
  });
});

describe("queryEvents", () => {
  test("sends GET /api/events with no params", async () => {
    const client = createClient();
    const response = { events: [], count: 0 };

    mockFetch.mockImplementation(() =>
      Promise.resolve(new Response(JSON.stringify(response), { status: 200 })),
    );

    const result = await client.queryEvents();

    expect(result).toEqual(response);
    const [url] = mockFetch.mock.calls[0] as [string];
    expect(url).toBe(`${MOCK_BASE_URL}/api/events`);
  });

  test("sends query params when provided", async () => {
    const client = createClient();
    const response = { events: [], count: 0 };

    mockFetch.mockImplementation(() =>
      Promise.resolve(new Response(JSON.stringify(response), { status: 200 })),
    );

    await client.queryEvents({ entity_id: "user-123", limit: 10, event_type: "user.signup" });

    const [url] = mockFetch.mock.calls[0] as [string];
    const parsed = new URL(url);
    expect(parsed.searchParams.get("entity_id")).toBe("user-123");
    expect(parsed.searchParams.get("limit")).toBe("10");
    expect(parsed.searchParams.get("event_type")).toBe("user.signup");
  });

  test("omits undefined params", async () => {
    const client = createClient();
    mockFetch.mockImplementation(() =>
      Promise.resolve(new Response(JSON.stringify({ events: [], count: 0 }), { status: 200 })),
    );

    await client.queryEvents({ entity_id: "user-123", limit: undefined });

    const [url] = mockFetch.mock.calls[0] as [string];
    const parsed = new URL(url);
    expect(parsed.searchParams.get("entity_id")).toBe("user-123");
    expect(parsed.searchParams.has("limit")).toBe(false);
  });
});

describe("error handling", () => {
  test("throws AllSourceError on non-2xx response", async () => {
    const client = createClient();
    mockFetch.mockImplementation(() =>
      Promise.resolve(
        new Response(JSON.stringify({ error: "Unauthorized" }), { status: 401 }),
      ),
    );

    try {
      await client.getHealth();
      expect(true).toBe(false); // should not reach
    } catch (err) {
      expect(err).toBeInstanceOf(AllSourceError);
      const error = err as AllSourceError;
      expect(error.status).toBe(401);
      expect(error.body).toEqual({ error: "Unauthorized" });
    }
  });

  test("AllSourceError includes status text", async () => {
    const client = createClient();
    mockFetch.mockImplementation(() =>
      Promise.resolve(
        new Response("Not Found", { status: 404, statusText: "Not Found" }),
      ),
    );

    try {
      await client.getHealth();
      expect(true).toBe(false);
    } catch (err) {
      const error = err as AllSourceError;
      expect(error.message).toContain("404");
      expect(error.message).toContain("Not Found");
    }
  });
});
