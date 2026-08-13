import { type NextRequest, NextResponse } from "next/server";

/**
 * Demo seeding lives on Query Service, not the branded Control Plane gateway.
 * Keep this route-specific proxy separate from the generic `/api/v1` proxy:
 * other v1 calls intentionally use the gateway, while `/demo/seed` must reach
 * Query Service directly.
 */
function getQueryServiceUrl(): string {
  return (
    process.env.QUERY_SERVICE_URL ||
    (process.env.NODE_ENV === "production"
      ? "https://allsource-query.fly.dev"
      : "http://localhost:3902")
  );
}

export async function POST(request: NextRequest): Promise<NextResponse> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
  };
  const token = request.cookies.get("auth_token")?.value;
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }

  try {
    const response = await fetch(`${getQueryServiceUrl()}/api/v1/demo/seed`, {
      method: "POST",
      headers,
      body: "{}",
    });
    const body = await response.text();

    return new NextResponse(body, {
      status: response.status,
      headers: {
        "content-type": response.headers.get("content-type") || "application/json",
      },
    });
  } catch {
    return NextResponse.json(
      { error: "Demo service is unavailable. Try again shortly." },
      { status: 502 }
    );
  }
}
