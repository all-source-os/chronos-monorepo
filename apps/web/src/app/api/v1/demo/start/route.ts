import { type NextRequest, NextResponse } from "next/server";

/**
 * Proxy demo account creation to the Control Plane (not Query Service).
 *
 * The generic /api/v1/[...path] catch-all routes to the Query Service,
 * but /api/v1/demo/start lives on the Control Plane. This specific route
 * takes precedence over the catch-all.
 */

function getControlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || process.env.CONTROL_PLANE_URL || "http://localhost:3901";
}

export async function POST(request: NextRequest) {
  const url = `${getControlPlaneUrl()}/api/v1/demo/start`;

  try {
    const response = await fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
    });

    const body = await response.text();
    return new NextResponse(body, {
      status: response.status,
      headers: { "content-type": "application/json" },
    });
  } catch {
    return NextResponse.json(
      { error: "Failed to reach Control Plane for demo provisioning" },
      { status: 502 }
    );
  }
}
