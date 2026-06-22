"use client";

import useSWR from "swr";
import { apiClient, type EventTypeInfo, type EventTypesListResponse } from "@/lib/api/client";

interface EventTypesParams {
  limit?: number;
  offset?: number;
}

const fetcher = async (key: string) => {
  const params: EventTypesParams = {};
  const searchParams = new URLSearchParams(key.split("?")[1] || "");

  if (searchParams.get("limit")) params.limit = parseInt(searchParams.get("limit")!, 10);
  if (searchParams.get("offset")) params.offset = parseInt(searchParams.get("offset")!, 10);

  const response = await apiClient.listEventTypes(params);
  if (response.error) throw new Error(response.error.message);
  return response.data;
};

export function useEventTypes(params?: EventTypesParams) {
  const queryParams: Record<string, string> = {};
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) queryParams[key] = String(value);
    }
  }

  const queryString =
    Object.keys(queryParams).length > 0 ? `?${new URLSearchParams(queryParams).toString()}` : "";

  const { data, error, isLoading, isValidating, mutate } = useSWR(
    `/api/event-types${queryString}`,
    fetcher,
    {
      revalidateOnFocus: false,
      dedupingInterval: 10000,
    }
  );

  // apiClient.request() already unwrapped the `{ data: [...] }` envelope, so
  // `data` is the EventTypeInfo[] array, not the wrapper — `data.data` was
  // undefined, rendering the event-types list empty despite rows.
  const raw = data as EventTypeInfo[] | EventTypesListResponse | undefined;
  const list: EventTypeInfo[] = Array.isArray(raw) ? raw : ((raw?.data ?? []) as EventTypeInfo[]);

  return {
    eventTypes: list,
    total: Array.isArray(raw) ? raw.length : (raw?.total ?? list.length),
    isLoading,
    isValidating,
    error: error?.message,
    refresh: () => mutate(),
  };
}
