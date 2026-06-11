import { type NextRequest, NextResponse } from "next/server";

// In-place plan change for an existing subscriber. Forwards straight to the
// control plane (which owns LemonSqueezy) with the user's JWT, same as checkout.
function controlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  const token = request.cookies.get("auth_token")?.value;
  const headers: Record<string, string> = { "content-type": "application/json" };
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }

  const body = await request.text();

  try {
    const res = await fetch(`${controlPlaneUrl()}/api/v1/billing/change-plan`, {
      method: "POST",
      headers,
      body,
    });
    const text = await res.text();
    return new NextResponse(text, {
      status: res.status,
      headers: { "content-type": res.headers.get("content-type") || "application/json" },
    });
  } catch {
    return NextResponse.json({ error: "Failed to reach billing service" }, { status: 502 });
  }
}
