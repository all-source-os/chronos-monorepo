"use client";

import useSWR from "swr";
import { useTimeTravelOptional } from "@/hooks/use-time-travel";
import { apiClient } from "@/lib/api/client";

interface DashboardStats {
  events: {
    used: number;
    quota: number;
    percentage: number;
  };
  queries: {
    used: number;
    quota: number;
    percentage: number;
  };
  projections: {
    count: number;
    active: number;
  };
  latency: {
    p99_us: number;
    formatted: string;
  };
  storage: {
    bytes: number;
    formatted: string;
  };
  isHistorical?: boolean;
  asOf?: string | null;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const val = bytes / 1024 ** i;
  return `${val < 10 ? val.toFixed(1) : Math.round(val)} ${units[i]}`;
}

function formatLatency(us: number): string {
  if (us < 1000) return `${us.toFixed(1)}μs`;
  return `${(us / 1000).toFixed(1)}ms`;
}

const DEFAULT_STATS: DashboardStats = {
  events: { used: 0, quota: 10000, percentage: 0 },
  queries: { used: 0, quota: 10000, percentage: 0 },
  projections: { count: 0, active: 0 },
  latency: { p99_us: 0, formatted: "—" },
  storage: { bytes: 0, formatted: "—" },
};

async function fetchDashboardStats(asOfIso: string | null): Promise<DashboardStats> {
  // Set the asOf on the client for time travel queries
  apiClient.setAsOf(asOfIso);

  // Fetch all data in parallel
  const [usageResponse, projectionsResponse, metricsResponse] = await Promise.all([
    apiClient.getTenantUsage(),
    apiClient.listProjections(),
    apiClient.getMetrics(),
  ]);

  // Build stats object with fallbacks
  const stats: DashboardStats = { ...DEFAULT_STATS };

  // Usage data
  if (usageResponse.data) {
    stats.events = usageResponse.data.events;
    stats.queries = usageResponse.data.queries;
  }

  // Projections count
  if (projectionsResponse.data) {
    const projections = projectionsResponse.data;
    stats.projections = {
      count: projections.length,
      active: projections.filter((p) => p.status === "running").length,
    };
  }

  // Extract real metrics from backend response
  if (metricsResponse.data) {
    const backend = (metricsResponse.data as unknown as Record<string, unknown>).backend as Record<string, unknown> | undefined;
    const p99 = (backend?.p99_latency_us as number) ?? (backend?.latency_p99_us as number) ?? 0;
    stats.latency = { p99_us: p99, formatted: p99 > 0 ? formatLatency(p99) : "—" };

    const storageBytes = (backend?.storage_bytes as number) ?? (backend?.disk_usage_bytes as number) ?? 0;
    stats.storage = { bytes: storageBytes, formatted: storageBytes > 0 ? formatBytes(storageBytes) : "—" };
  }

  // Include time travel metadata
  stats.isHistorical = asOfIso !== null;
  stats.asOf = asOfIso;

  return stats;
}

export function useDashboardStats() {
  const { asOfIso, isHistorical } = useTimeTravelOptional();

  const { data, error, isLoading, mutate } = useSWR(
    // Include asOfIso in the key so SWR refetches when time travel changes
    asOfIso ? `/dashboard/stats?as_of=${asOfIso}` : "/dashboard/stats",
    () => fetchDashboardStats(asOfIso),
    {
      revalidateOnFocus: false,
      dedupingInterval: 30000, // Cache for 30 seconds
      fallbackData: DEFAULT_STATS,
      onError: (err) => {
        console.warn("Failed to fetch dashboard stats:", err.message);
      },
    }
  );

  return {
    stats: data || DEFAULT_STATS,
    isLoading,
    isHistorical,
    error: error?.message,
    refresh: () => mutate(),
  };
}
