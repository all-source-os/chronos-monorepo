// Realistic sample data for marketing compositions.
// Numbers match the pricing memo and production benchmarks.

export const BENCHMARKS = {
  eventsPerSec: 469_000,
  queryLatencyUs: 11.9,
  mcpTools: 43,
  dockerImageMb: 15.7,
} as const;

export const TIERS = [
  { name: "Developer", price: "Free", events: "100K/mo" },
  { name: "Pro", price: "$29/mo", events: "1M/mo" },
  { name: "Growth", price: "$79/mo", events: "10M/mo" },
  { name: "Enterprise", price: "Custom", events: "Unlimited" },
] as const;

export const SAMPLE_EVENTS = [
  { id: "evt-001", type: "user.signup", entity: "usr-8f3a", ts: "2026-04-20T09:14:22Z", payload: { email: "alice@example.com", plan: "pro" } },
  { id: "evt-002", type: "order.placed", entity: "ord-12c7", ts: "2026-04-20T09:15:01Z", payload: { total: 149.99, items: 3 } },
  { id: "evt-003", type: "payment.settled", entity: "pay-9b21", ts: "2026-04-20T09:15:03Z", payload: { amount: "0.0001", network: "base", tx: "0xabc..." } },
  { id: "evt-004", type: "agent.query", entity: "agt-d4e1", ts: "2026-04-20T09:15:10Z", payload: { tool: "query_events", tokens: 1200 } },
  { id: "evt-005", type: "projection.updated", entity: "prj-7fa2", ts: "2026-04-20T09:15:15Z", payload: { name: "order-totals", events_processed: 18225 } },
  { id: "evt-006", type: "schema.registered", entity: "sch-3c90", ts: "2026-04-20T09:15:22Z", payload: { name: "order.placed", version: 2 } },
  { id: "evt-007", type: "user.login", entity: "usr-8f3a", ts: "2026-04-20T09:16:01Z", payload: { provider: "github", ip: "192.168.1.1" } },
  { id: "evt-008", type: "order.shipped", entity: "ord-12c7", ts: "2026-04-20T09:17:30Z", payload: { carrier: "fedex", tracking: "FX123456789" } },
] as const;
