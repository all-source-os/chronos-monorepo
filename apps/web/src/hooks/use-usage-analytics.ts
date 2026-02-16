import useSWR from "swr";

import { apiClient } from "@/lib/api/client";

export function useUsageAnalytics(range?: string) {
  const key = range ? `/api/tenants/me/analytics?range=${range}` : "/api/tenants/me/analytics";

  const { data, error, isLoading } = useSWR(
    key,
    async () => {
      const response = await apiClient.getUsageAnalytics(range ? { range } : undefined);
      if (response.error) throw new Error(response.error.message);
      return response.data;
    },
    { revalidateOnFocus: false }
  );

  return {
    analytics: data ?? null,
    eventTypeDistribution: data?.event_type_distribution ?? [],
    topEntityIds: data?.top_entity_ids ?? [],
    ingestionRate: data?.ingestion_rate ?? [],
    range: data?.range ?? range ?? "7d",
    isLoading,
    error: error?.message,
  };
}
