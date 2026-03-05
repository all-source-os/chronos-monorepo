import { NextResponse } from "next/server";

function getServiceHealthUrls(): Record<string, string> {
  const queryServiceUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3902";
  const coreUrl = process.env.CORE_URL || "http://localhost:3900";
  const controlPlaneUrl = process.env.CONTROL_PLANE_URL || "http://localhost:3901";

  return {
    core: `${coreUrl}/health`,
    "query-service": `${queryServiceUrl}/api/health`,
    "control-plane": `${controlPlaneUrl}/health`,
  };
}

export async function GET(_request: Request, { params }: { params: Promise<{ service: string }> }) {
  const { service } = await params;
  const healthUrl = getServiceHealthUrls()[service];

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
