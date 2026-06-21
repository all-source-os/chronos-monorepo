import { NextResponse } from "next/server";

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

// Synthetic end-to-end auth check. The CP "auth" heartbeat only probes the
// auth *process* health, which stayed green while logins failed — because the
// break was the session-token *validation path*, not the auth service.
//
// This exercises the exact endpoint the login callback validates against:
// 401 (no auth) = path reachable and enforcing → login works.
// 404 = path not routed (the failure that showed every login "Session expired").
// 5xx / timeout = backend down.
async function probeLoginAuth(): Promise<ServiceHeartbeat> {
  const url = `${getQueryServiceUrl()}/api/v1/auth/me`;
  const start = Date.now();
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);
    const res = await fetch(url, { signal: controller.signal, cache: "no-store" });
    clearTimeout(timeout);

    const healthy = res.status === 401;
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
          ? "validation endpoint not routed (404) — logins would fail with 'Session expired'"
          : `unexpected HTTP ${res.status} (expected 401)`,
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

export async function GET() {
  const cpUrl = getControlPlaneUrl();

  // Run the synthetic login check in parallel with the CP feed so the auth
  // status is always present even if the CP feed is slow/down.
  const loginCheckPromise = probeLoginAuth();

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);

    const response = await fetch(`${cpUrl}/api/v1/status/services`, {
      signal: controller.signal,
      cache: "no-store",
    });
    clearTimeout(timeout);

    const loginCheck = await loginCheckPromise;

    if (!response.ok) {
      return NextResponse.json(
        { services: [loginCheck], error: `upstream ${response.status}` },
        { status: 200 }
      );
    }

    const data = await response.json();
    const upstream: ServiceHeartbeat[] = Array.isArray(data?.services) ? data.services : [];
    return NextResponse.json({ ...data, services: [...upstream, loginCheck] });
  } catch (err) {
    const loginCheck = await loginCheckPromise;
    return NextResponse.json(
      { services: [loginCheck], error: err instanceof Error ? err.message : String(err) },
      { status: 200 }
    );
  }
}
