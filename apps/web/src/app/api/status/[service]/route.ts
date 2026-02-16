import { NextResponse } from "next/server";

const QUERY_SERVICE_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
const CORE_URL = process.env.CORE_URL || "http://localhost:3900";
const CONTROL_PLANE_URL = process.env.CONTROL_PLANE_URL || "http://localhost:3901";

const SERVICE_HEALTH_URLS: Record<string, string> = {
  core: `${CORE_URL}/health`,
  "query-service": `${QUERY_SERVICE_URL}/api/health`,
  "control-plane": `${CONTROL_PLANE_URL}/health`,
};

export async function GET(_request: Request, { params }: { params: Promise<{ service: string }> }) {
  const { service } = await params;
  const healthUrl = SERVICE_HEALTH_URLS[service];

  if (!healthUrl) {
    return NextResponse.json({ error: "Unknown service" }, { status: 404 });
  }

  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 10_000);

    const response = await fetch(healthUrl, {
      signal: controller.signal,
      cache: "no-store",
    });
    clearTimeout(timeout);

    if (response.ok) {
      const data = await response.json();
      return NextResponse.json({ status: "operational", data });
    }
    return NextResponse.json({ status: "degraded", code: response.status }, { status: 200 });
  } catch {
    return NextResponse.json({ status: "down" }, { status: 503 });
  }
}
