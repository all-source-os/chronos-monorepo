"use client";

import useSWR from "swr";
import { apiClient, type SchemaEnforcementMode } from "@/lib/api/client";

const DEFAULT_MODE: SchemaEnforcementMode = "permissive";

async function fetchMode(): Promise<SchemaEnforcementMode> {
  const res = await apiClient.getSchemaEnforcement();
  return res.data?.schema_enforcement ?? DEFAULT_MODE;
}

/**
 * Reads and updates the tenant's schema-enforcement mode (Gap 3 toggle).
 * Backed by the gateway endpoint /api/tenant/schema-enforcement, which proxies
 * Core's per-tenant setting.
 */
export function useSchemaEnforcement() {
  const { data, error, isLoading, mutate } = useSWR("/tenant/schema-enforcement", fetchMode, {
    revalidateOnFocus: false,
    dedupingInterval: 30000,
    fallbackData: DEFAULT_MODE,
  });

  const mode = data ?? DEFAULT_MODE;

  const setMode = async (next: SchemaEnforcementMode): Promise<boolean> => {
    const previous = mode;
    // Optimistic update.
    mutate(next, false);
    const res = await apiClient.setSchemaEnforcement(next);
    if (res.error) {
      // Roll back on failure.
      mutate(previous, false);
      return false;
    }
    mutate(res.data?.schema_enforcement ?? next, false);
    return true;
  };

  return { mode, isLoading, error: error?.message as string | undefined, setMode };
}
