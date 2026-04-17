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

export async function GET() {
  const cpUrl = getControlPlaneUrl();

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 5_000);

    const response = await fetch(`${cpUrl}/api/v1/status/services`, {
      signal: controller.signal,
      cache: "no-store",
    });
    clearTimeout(timeout);

    if (!response.ok) {
      return NextResponse.json(
        { services: [], error: `upstream ${response.status}` },
        { status: 502 }
      );
    }

    const data = await response.json();
    return NextResponse.json(data);
  } catch (err) {
    return NextResponse.json(
      { services: [], error: err instanceof Error ? err.message : String(err) },
      { status: 502 }
    );
  }
}
