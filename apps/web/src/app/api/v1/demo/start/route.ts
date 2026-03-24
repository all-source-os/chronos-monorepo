import { type NextRequest, NextResponse } from "next/server";

/**
 * Proxy demo account creation to the Auth Service.
 *
 * Falls back to Control Plane if AUTH_SERVICE_URL is not set (migration compat).
 */

function getAuthServiceUrl(): string {
  return process.env.AUTH_SERVICE_URL || process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3903";
}

export async function POST(request: NextRequest) {
  const url = `${getAuthServiceUrl()}/api/auth/demo/start`;

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
      { error: "Failed to reach Auth Service for demo provisioning" },
      { status: 502 }
    );
  }
}
