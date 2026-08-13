import type { ReplayProgress, ReplayStatus } from "@/lib/api/client";

const KNOWN_STATUSES = new Set<ReplayStatus>([
  "pending",
  "running",
  "completed",
  "failed",
  "cancelled",
]);

export function normalizeReplayStatus(value: unknown): ReplayStatus {
  if (typeof value !== "string") return "unknown";
  const normalized = value.toLowerCase() as ReplayStatus;
  return KNOWN_STATUSES.has(normalized) ? normalized : "unknown";
}

export function normalizeReplay(value: ReplayProgress): ReplayProgress {
  return { ...value, status: normalizeReplayStatus(value.status) };
}
