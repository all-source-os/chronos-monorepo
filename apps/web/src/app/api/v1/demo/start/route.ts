import { type NextRequest, NextResponse } from "next/server";

/**
 * Proxy demo account creation.
 *
 * When AUTH_SERVICE_URL is set, calls the better-auth-rs service at /api/auth/demo/start.
 * Otherwise, falls back to the Control Plane at /api/v1/demo/start.
 */

function getDemoUrl(): string {
  const authService = process.env.AUTH_SERVICE_URL;
  if (authService) {
    return `${authService}/api/auth/demo/start`;
  }
  const controlPlane = process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
  return `${controlPlane}/api/v1/demo/start`;
}

export async function POST(request: NextRequest) {
  const url = getDemoUrl();

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
