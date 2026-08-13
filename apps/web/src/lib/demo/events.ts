import type { CreateEventRequest, Event } from "@/lib/api/client";

const DEMO_EVENT_COUNT = 60;

const EVENT_SPECS = [
  {
    event_type: "log.error",
    entity: "checkout-api",
    messages: [
      "Payment provider returned HTTP 500",
      "API timeout while confirming order",
      "Database query exceeded latency budget",
    ],
  },
  {
    event_type: "metric.memory",
    entity: "worker-pool",
    messages: [
      "Memory pressure reached 82 percent",
      "Heap growth suggests a memory leak",
      "Worker memory returned below threshold",
    ],
  },
  {
    event_type: "metric.latency",
    entity: "search-api",
    messages: [
      "P95 latency spike detected",
      "Vector query completed inside budget",
      "Database response slowed search requests",
    ],
  },
  {
    event_type: "user.signup",
    entity: "growth-funnel",
    messages: [
      "New Studio workspace created",
      "Signup completed after email verification",
      "User selected developer onboarding",
    ],
  },
] as const;

export function buildDemoEvents(now = new Date()): CreateEventRequest[] {
  return Array.from({ length: DEMO_EVENT_COUNT }, (_, index) => {
    const spec = EVENT_SPECS[index % EVENT_SPECS.length]!;
    const message = spec.messages[index % spec.messages.length];
    const timestamp = new Date(now.getTime() - (DEMO_EVENT_COUNT - index) * 1_500).toISOString();

    return {
      entity_id: `${spec.entity}-${String(index % 5).padStart(2, "0")}`,
      event_type: spec.event_type,
      payload: {
        message,
        service: spec.entity,
        environment: "demo",
        duration_ms: 18 + ((index * 37) % 720),
        demo_sequence: index + 1,
        demo_timestamp: timestamp,
      },
    };
  });
}

export function normalizeDemoEvents(value: unknown): Event[] {
  if (!value || typeof value !== "object") return [];
  const payload = value as Record<string, unknown>;
  const raw = Array.isArray(payload.events)
    ? payload.events
    : Array.isArray(payload.data)
      ? payload.data
      : [];

  return raw.filter((event): event is Event => {
    if (!event || typeof event !== "object") return false;
    const candidate = event as Partial<Event>;
    return (
      typeof candidate.id === "string" &&
      typeof candidate.entity_id === "string" &&
      typeof candidate.event_type === "string" &&
      typeof candidate.timestamp === "string" &&
      !!candidate.payload &&
      typeof candidate.payload === "object"
    );
  });
}
