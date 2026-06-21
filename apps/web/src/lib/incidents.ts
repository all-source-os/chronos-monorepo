// Durable, event-sourced incident log for the /status page. Incidents are
// stored as `status.incident` events in AllSource itself (one stream per
// service: entity `incident:<service>`), so history survives redeploys and is
// auditable like everything else in the product.
//
// Requires a monitoring API key (INCIDENT_API_KEY) — a service-account key for
// the tenant that should own the status stream. Without it, logging is a no-op
// and the incident history is empty (the feature degrades gracefully).
//
// IP info is deliberately limited: only a redacted network prefix (IPv4 /24,
// IPv6 /48) is stored, never a full client address.

interface IncidentConfig {
  url: string;
  key: string;
}

function getConfig(): IncidentConfig | null {
  const key = process.env.INCIDENT_API_KEY;
  if (!key) return null;
  const url =
    process.env.QUERY_SERVICE_URL ||
    (process.env.NODE_ENV === "production"
      ? "https://allsource-query.fly.dev"
      : "http://localhost:3902");
  return { url, key };
}

/** Reduce a client IP to a coarse network prefix — never store a full address. */
export function redactIp(raw: string | null | undefined): string | null {
  if (!raw) return null;
  const ip = raw.split(",")[0]?.trim();
  if (!ip) return null;
  if (ip.includes(":")) {
    // IPv6 → /48 (first three hextets)
    const hextets = ip.split(":").filter(Boolean).slice(0, 3);
    return hextets.length ? `${hextets.join(":")}::/48` : null;
  }
  const octets = ip.split(".");
  return octets.length === 4 ? `${octets[0]}.${octets[1]}.${octets[2]}.0/24` : null;
}

async function ingest(cfg: IncidentConfig, entityId: string, payload: unknown): Promise<void> {
  await fetch(`${cfg.url}/api/v1/events`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-API-Key": cfg.key },
    cache: "no-store",
    body: JSON.stringify({ event_type: "status.incident", entity_id: entityId, payload }),
  });
}

interface IncidentEvent {
  entity_id?: string;
  timestamp?: string;
  payload?: {
    status?: "open" | "resolved";
    service?: string;
    at?: string;
    error?: string;
    observed_ip_prefix?: string | null;
  };
}

async function latestEvent(cfg: IncidentConfig, entityId: string): Promise<IncidentEvent | null> {
  const res = await fetch(
    `${cfg.url}/api/v1/events/query?entity_id=${encodeURIComponent(entityId)}&limit=1&order=desc`,
    { headers: { "X-API-Key": cfg.key }, cache: "no-store" }
  );
  if (!res.ok) return null;
  const data = (await res.json()) as { events?: IncidentEvent[] };
  return data.events?.[0] ?? null;
}

/**
 * Record a synthetic-check result. Opens an incident on healthy→unhealthy and
 * resolves it on unhealthy→healthy, by appending to the per-service event
 * stream. Stateless and idempotent-ish across serverless instances (reads the
 * latest event before deciding). Best-effort — never throws into the caller.
 */
export async function recordCheck(
  service: string,
  healthy: boolean,
  observedIpPrefix: string | null
): Promise<void> {
  const cfg = getConfig();
  if (!cfg) return;
  const entity = `incident:${service}`;
  try {
    const last = await latestEvent(cfg, entity);
    const isOpen = last?.payload?.status === "open";
    if (!healthy && !isOpen) {
      await ingest(cfg, entity, {
        status: "open",
        service,
        at: new Date().toISOString(),
        error: undefined,
        observed_ip_prefix: observedIpPrefix,
      });
    } else if (healthy && isOpen) {
      await ingest(cfg, entity, { status: "resolved", service, at: new Date().toISOString() });
    }
  } catch {
    // never break the status feed
  }
}

export interface Incident {
  service: string;
  started_at: string;
  resolved_at: string | null;
  observed_ip_prefix?: string | null;
}

/** Fold the status.incident event stream into a list of incidents (newest first). */
export async function getRecentIncidents(limit = 20): Promise<Incident[]> {
  const cfg = getConfig();
  if (!cfg) return [];
  try {
    const res = await fetch(
      `${cfg.url}/api/v1/events/query?event_type=status.incident&limit=200&order=desc`,
      { headers: { "X-API-Key": cfg.key }, cache: "no-store" }
    );
    if (!res.ok) return [];
    const data = (await res.json()) as { events?: IncidentEvent[] };
    const events = (data.events ?? []).slice().reverse(); // oldest → newest

    const open: Record<string, Incident> = {};
    const incidents: Incident[] = [];
    for (const e of events) {
      const p = e.payload ?? {};
      const svc = p.service ?? e.entity_id?.replace("incident:", "") ?? "unknown";
      if (p.status === "open") {
        const inc: Incident = {
          service: svc,
          started_at: p.at ?? e.timestamp ?? "",
          resolved_at: null,
          observed_ip_prefix: p.observed_ip_prefix ?? null,
        };
        open[svc] = inc;
        incidents.push(inc);
      } else if (p.status === "resolved" && open[svc]) {
        open[svc].resolved_at = p.at ?? e.timestamp ?? null;
        delete open[svc];
      }
    }
    return incidents.slice(-limit).reverse();
  } catch {
    return [];
  }
}
