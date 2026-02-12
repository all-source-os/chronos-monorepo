"use client";

import useSWR from "swr";
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
}

const DEFAULT_STATS: DashboardStats = {
  events: { used: 0, quota: 10000, percentage: 0 },
  queries: { used: 0, quota: 10000, percentage: 0 },
  projections: { count: 0, active: 0 },
  latency: { p99_us: 11900, formatted: "11.9μs" },
};

async function fetchDashboardStats(): Promise<DashboardStats> {
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

  return stats;
}

export function useDashboardStats() {
  const { data, error, isLoading, mutate } = useSWR(
    "/dashboard/stats",
    fetchDashboardStats,
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
    error: error?.message,
    refresh: () => mutate(),
  };
}
