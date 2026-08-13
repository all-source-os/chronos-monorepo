import { type NextRequest, NextResponse } from "next/server";
import { buildDemoEvents } from "@/lib/demo/events";

/**
 * Seed a compact sample into the caller's own workspace. Older behavior called
 * Core's public global seed endpoint, which wrote to tenant `default`; signed-in
 * users then saw a success response and an empty tenant-scoped demo.
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
    const response = await fetch(`${getQueryServiceUrl()}/api/v1/events/batch`, {
      method: "POST",
      headers,
      body: JSON.stringify({ events: buildDemoEvents() }),
    });
    const body = await response.text();

    if (!response.ok) {
      return new NextResponse(body, {
        status: response.status,
        headers: { "content-type": response.headers.get("content-type") || "application/json" },
      });
    }

    const parsed = JSON.parse(body) as { count?: number; data?: unknown[] };
    const eventCount = parsed.count ?? parsed.data?.length ?? 0;

    return NextResponse.json({
      seeded: true,
      event_count: eventCount,
      message: "Sample events added to your workspace.",
    });
  } catch {
    return NextResponse.json(
      { error: "Demo service is unavailable. Try again shortly." },
      { status: 502 }
    );
  }
}
