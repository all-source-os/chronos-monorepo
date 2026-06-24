import { type NextRequest, NextResponse } from "next/server";
import { recordCheck, redactIp } from "@/lib/incidents";

// Proxy to Control Plane's event-sourced status feed. The page in
// /status calls this so there's no client-side CORS concern and we get
// server-side caching. CP is the source of truth — it probes every
// backend every 10s and writes a service.heartbeat event to Core.
//
// CONTROL_PLANE_URL is CP's public URL (e.g. https://api.all-source.xyz).
// Falls back to the default fly.dev hostname so this works out of the
// box in environments that don't set the env var explicitly.
function getControlPlaneUrl(): string {
  return (
    process.env.CONTROL_PLANE_URL ||
    process.env.NEXT_PUBLIC_CONTROL_PLANE_URL ||
    "https://api.all-source.xyz"
  );
}

// The Query Service validates session tokens (the login callback hits
// /api/v1/auth/me here). Same resolution as the auth callback.
function getQueryServiceUrl(): string {
  return (
    process.env.QUERY_SERVICE_URL ||
    (process.env.NODE_ENV === "production"
      ? "https://allsource-query.fly.dev"
      : "http://localhost:3902")
  );
}

interface ServiceHeartbeat {
  service: string;
  // Mirrors the CP feed's three values. The /status page renders "stale" as
  // "Degraded" — its semantic is "probe missing / not confirmed down", which is
  // exactly how we surface the login signal when no STATUS_MONITOR_TOKEN is set
  // (the end-to-end auth check is *unknown*, not failing).
  status: "healthy" | "unhealthy" | "stale";
  latency_ms: number;
  last_seen: string;
  age_seconds: number;
  error?: string;
  probed_url?: string;
}

// A persistent, NON-MUTATING credential for the end-to-end login probe.
//
// Set STATUS_MONITOR_TOKEN in Vercel (for www.all-source.xyz) to a long-lived
// service token (a `serviceaccount`-role API key minted for a dedicated
// status-monitor tenant). It is read at request time — Vercel functions are
// stateless, so we never cache or mint anything.
//
// HARD RULE: the status path must NEVER call a side-effectful endpoint. The
// previous implementation minted a probe session via the Control Plane demo
// onboarding flow, which creates a "Demo User" tenant (is_demo:true) on every
// call — and its module-level cache dies on cold starts, so each poll littered
// /tenants with a new demo tenant. That mint is gone. Do NOT reintroduce the
// demo onboarding call (or any other mutating endpoint) into this
// liveness/status code path.
function getMonitorToken(): string | null {
  const token = process.env.STATUS_MONITOR_TOKEN?.trim();
  return token && token.length > 0 ? token : null;
}

// End-to-end login check. Two non-mutating modes, never minting:
//
//  1. STATUS_MONITOR_TOKEN set → validate that REAL token against the same
//     endpoint the dashboard login callback + session check use (QS
//     /api/v1/auth/me). 200 = a real token round-trips → login works.
//     401/404/5xx = it does NOT — exactly the failure the process-only "auth"
//     heartbeat missed (an earlier probe only checked the endpoint returned 401
//     for no-auth, which stayed green while real sessions 404'd against a
//     misrouted host and users were silently logged out).
//
//  2. STATUS_MONITOR_TOKEN unset → we can't prove the authed round-trip without
//     minting (which is forbidden — see getMonitorToken). Degrade gracefully:
//     liveness-only against QS GET /health (unauthenticated) and mark the
//     end-to-end auth signal as UNKNOWN (status "stale" → rendered "Degraded"),
//     never as a failure and never creating anything.
async function probeLoginAuth(): Promise<ServiceHeartbeat> {
  const token = getMonitorToken();
  const start = Date.now();

  if (!token) {
    // Liveness-only fallback: is the auth/session host even up? No token, no
    // round-trip semantics, nothing created. The authed signal is reported as
    // unknown (stale/Degraded), not down.
    const url = `${getQueryServiceUrl()}/health`;
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 5_000);
      const res = await fetch(url, { signal: controller.signal, cache: "no-store" });
      clearTimeout(timeout);

      const up = res.ok;
      return {
        service: "login",
        // up but unverified end-to-end → "stale" (Degraded); host down → "unhealthy".
        status: up ? "stale" : "unhealthy",
        latency_ms: Date.now() - start,
        last_seen: new Date().toISOString(),
        age_seconds: 0,
        probed_url: url,
        error: up
          ? "end-to-end auth check unknown — set STATUS_MONITOR_TOKEN to validate a real session (liveness only)"
          : `auth/session host unreachable (HTTP ${res.status})`,
      };
    } catch (err) {
      return {
        service: "login",
        status: "unhealthy",
        latency_ms: Date.now() - start,
        last_seen: new Date().toISOString(),
        age_seconds: 0,
        probed_url: url,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  const url = `${getQueryServiceUrl()}/api/v1/auth/me`;
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);
    const res = await fetch(url, {
      headers: { Authorization: `Bearer ${token}` },
      signal: controller.signal,
      cache: "no-store",
    });
    clearTimeout(timeout);

    const healthy = res.status === 200;
    return {
      service: "login",
      status: healthy ? "healthy" : "unhealthy",
      latency_ms: Date.now() - start,
      last_seen: new Date().toISOString(),
      age_seconds: 0,
      probed_url: url,
      error: healthy
        ? undefined
        : res.status === 404
          ? "validation endpoint not routed (404) — logins fail with 'Session expired'"
          : res.status === 401
            ? "STATUS_MONITOR_TOKEN rejected (401) — the monitor token is expired/invalid, not a login outage"
            : `real session token rejected (HTTP ${res.status}, expected 200)`,
    };
  } catch (err) {
    return {
      service: "login",
      status: "unhealthy",
      latency_ms: Date.now() - start,
      last_seen: new Date().toISOString(),
      age_seconds: 0,
      probed_url: url,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export async function GET(request: NextRequest) {
  const cpUrl = getControlPlaneUrl();
  // Limited IP info: only a redacted network prefix (/24 or /48) is ever stored.
  const ipPrefix = redactIp(request.headers.get("x-forwarded-for"));

  // Run the synthetic login check in parallel with the CP feed so the auth
  // status is always present even if the CP feed is slow/down.
  const loginCheck = await probeLoginAuth();

  let services: ServiceHeartbeat[] = [loginCheck];
  let rest: Record<string, unknown> = {};
  let topError: string | undefined;

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);
    const response = await fetch(`${cpUrl}/api/v1/status/services`, {
      signal: controller.signal,
      cache: "no-store",
    });
    clearTimeout(timeout);

    if (response.ok) {
      const data = (await response.json()) as { services?: ServiceHeartbeat[] };
      const { services: upstream, ...other } = data;
      services = [...(Array.isArray(upstream) ? upstream : []), loginCheck];
      rest = other;
    } else {
      topError = `upstream ${response.status}`;
    }
  } catch (err) {
    topError = err instanceof Error ? err.message : String(err);
  }

  // Append/resolve incidents on status transitions, durably in the event log.
  // Best-effort and a no-op unless INCIDENT_API_KEY is configured.
  //
  // Only a CONFIRMED-down ("unhealthy") opens an incident. "stale" is degraded /
  // unknown (a missing CP heartbeat, or the login signal when no
  // STATUS_MONITOR_TOKEN is set) — not an outage — so it must NOT log an
  // incident, otherwise the default (token-unset) config would open a permanent
  // "login" incident on every poll.
  await Promise.allSettled(
    services.map((s) => recordCheck(s.service, s.status !== "unhealthy", ipPrefix))
  );

  return NextResponse.json({ ...rest, services, ...(topError ? { error: topError } : {}) });
}
