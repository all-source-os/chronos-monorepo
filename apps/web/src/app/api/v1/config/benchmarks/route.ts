import { NextResponse } from "next/server";

/**
 * Benchmark config lives on Query Service. The branded v1 gateway requires
 * authentication for this public endpoint, so proxy it directly at runtime.
 */
function getQueryServiceUrl(): string {
  return (
    process.env.QUERY_SERVICE_URL ||
    (process.env.NODE_ENV === "production"
      ? "https://allsource-query.fly.dev"
      : "http://localhost:3902")
  );
}

export async function GET(): Promise<NextResponse> {
  try {
    const response = await fetch(`${getQueryServiceUrl()}/api/v1/config/benchmarks`, {
      cache: "no-store",
    });
    const body = await response.text();

    return new NextResponse(body, {
      status: response.status,
      headers: {
        "content-type": response.headers.get("content-type") || "application/json",
        "cache-control": "public, max-age=300, stale-while-revalidate=3600",
      },
    });
  } catch {
    return NextResponse.json(
      { error: "Benchmark source is unavailable. Try again shortly." },
      { status: 502 }
    );
  }
}
