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
  isHistorical?: boolean;
  asOf?: string | null;
}

const DEFAULT_STATS: DashboardStats = {
  events: { used: 0, quota: 10000, percentage: 0 },
  queries: { used: 0, quota: 10000, percentage: 0 },
  projections: { count: 0, active: 0 },
  latency: { p99_us: 11900, formatted: "11.9μs" },
};

async function fetchDashboardStats(asOfIso: string | null): Promise<DashboardStats> {
  // Set the asOf on the client for time travel queries
  apiClient.setAsOf(asOfIso);

  // Fetch all data in parallel
  const [usageResponse, projectionsResponse] = await Promise.all([
    apiClient.getTenantUsage(),
    apiClient.listProjections(),
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

  // Latency is a static benchmark value for now
  // In future, could fetch from /api/metrics backend data
  stats.latency = { p99_us: 11900, formatted: "11.9μs" };

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
