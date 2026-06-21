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
  status: "healthy" | "unhealthy" | "stale";
  latency_ms: number;
  last_seen: string;
  age_seconds: number;
  error?: string;
  probed_url?: string;
}

// A real session token, cached so we mint at most ~once per warm instance
// lifetime (and on cold starts) instead of on every poll. Demo tokens are valid
// 7 days; re-mint when under an hour remains. (Minting via the demo flow does
// create a demo tenant — acceptably rare with this cache; enable DEMO_RESET to
// reap them.)
let monitorToken: { token: string; exp: number } | null = null;

async function getMonitorToken(cpUrl: string): Promise<string | null> {
  const now = Math.floor(Date.now() / 1000);
  if (monitorToken && monitorToken.exp - now > 3600) return monitorToken.token;
  try {
    const demo = await (
      await fetch(`${cpUrl}/api/v1/demo/start`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        cache: "no-store",
      })
    ).json();
    const login = await (
      await fetch(`${cpUrl}/api/v1/auth/login`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        cache: "no-store",
        body: JSON.stringify({ email: demo.email, password: demo.password }),
      })
    ).json();
    const token = login?.token;
    if (typeof token === "string") {
      const payload = JSON.parse(
        Buffer.from(token.split(".")[1] ?? "", "base64url").toString("utf-8")
      );
      monitorToken = { token, exp: Number(payload?.exp) || now + 3600 };
      return token;
    }
  } catch {
    // fall through to whatever we had cached
  }
  return monitorToken?.token ?? null;
}

// End-to-end login check: validate a REAL session token against the same
// endpoint the dashboard login callback + session check use. 200 = a real token
// round-trips → login works. 401/404/5xx = it does NOT — exactly the failure the
// process-only "auth" heartbeat missed (it only checked the auth service was up;
// an earlier version of this probe only checked the endpoint returned 401 for
// no-auth, which stayed green while real sessions 404'd against a misrouted host
// and users were silently logged out).
async function probeLoginAuth(cpUrl: string): Promise<ServiceHeartbeat> {
  const url = `${getQueryServiceUrl()}/api/v1/auth/me`;
  const start = Date.now();
  const token = await getMonitorToken(cpUrl);

  if (!token) {
    return {
      service: "login",
      status: "unhealthy",
      latency_ms: Date.now() - start,
      last_seen: new Date().toISOString(),
      age_seconds: 0,
      probed_url: url,
      error: "could not obtain a session token (login / demo mint failing)",
    };
  }

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
  const loginCheck = await probeLoginAuth(cpUrl);

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
  await Promise.allSettled(
    services.map((s) => recordCheck(s.service, s.status === "healthy", ipPrefix))
  );

  return NextResponse.json({ ...rest, services, ...(topError ? { error: topError } : {}) });
}
