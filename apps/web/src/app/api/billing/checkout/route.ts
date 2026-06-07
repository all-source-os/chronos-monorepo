import { type NextRequest, NextResponse } from "next/server";

// Checkout creation goes straight to the control plane (which owns the
// LemonSqueezy integration), NOT through the Query Service. The QS path issued a
// 301 redirect to the control plane built from an unset MGMT_PLANE_URL (→ a
// relative Location the Next proxy couldn't follow → 502), and an internal Fly
// URL wouldn't be reachable from Vercel anyway. This forwards the user's JWT as
// a Bearer token, which is what the control-plane AuthMiddleware expects.
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
    const res = await fetch(`${controlPlaneUrl()}/api/v1/billing/checkout`, {
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
