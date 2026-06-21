import { NextResponse } from "next/server";
import { getRecentIncidents } from "@/lib/incidents";

// GET /api/status/incidents — recent incidents for the /status history view,
// folded from the durable status.incident event stream in AllSource.
export async function GET() {
  const incidents = await getRecentIncidents(20);
  return NextResponse.json({ incidents });
}
