"use client";

import useSWR, { mutate } from "swr";
import { useTimeTravelOptional } from "@/hooks/use-time-travel";
import { apiClient, type CreateEventRequest, type ListEventsParams } from "@/lib/api/client";

const fetcher = async (key: string) => {
  const params: ListEventsParams = {};
  const searchParams = new URLSearchParams(key.split("?")[1] || "");

  if (searchParams.get("entity_id")) params.entity_id = searchParams.get("entity_id")!;
  if (searchParams.get("event_type")) params.event_type = searchParams.get("event_type")!;
  if (searchParams.get("event_type_prefix"))
    params.event_type_prefix = searchParams.get("event_type_prefix")!;
  if (searchParams.get("limit")) params.limit = parseInt(searchParams.get("limit")!, 10);
  if (searchParams.get("offset")) params.offset = parseInt(searchParams.get("offset")!, 10);
  if (searchParams.get("as_of")) params.as_of = searchParams.get("as_of")!;

  const response = await apiClient.listEvents(params);
  if (response.error) throw new Error(response.error.message);
  return response.data;
};

export function useEvents(params?: ListEventsParams) {
  const { asOfIso, isHistorical } = useTimeTravelOptional();

  // Build query params including time travel
  const queryParams: Record<string, string> = {};
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) queryParams[key] = String(value);
    }
  }
  // Add as_of from time travel context if not already specified
  if (asOfIso && !queryParams.as_of) {
    queryParams.as_of = asOfIso;
  }

  const queryString =
    Object.keys(queryParams).length > 0 ? `?${new URLSearchParams(queryParams).toString()}` : "";

  const { data, error, isLoading, isValidating } = useSWR(`/api/events${queryString}`, fetcher, {
    revalidateOnFocus: false,
    dedupingInterval: 5000,
  });

  const createEvent = async (event: CreateEventRequest) => {
    const response = await apiClient.createEvent(event);
    if (response.error) throw new Error(response.error.message);

    // Revalidate the events list
    mutate((key) => typeof key === "string" && key.startsWith("/api/events"));

    return response.data;
  };

  const createEventBatch = async (events: CreateEventRequest[]) => {
    const response = await apiClient.createEventBatch(events);
    if (response.error) throw new Error(response.error.message);

    // Revalidate the events list
    mutate((key) => typeof key === "string" && key.startsWith("/api/events"));

    return response.data;
  };

  return {
    events: data?.data || [],
    total: data?.count || 0,
    isLoading,
    isValidating,
    isHistorical,
    error: error?.message,
    createEvent,
    createEventBatch,
    refresh: () => mutate(`/api/events${queryString}`),
  };
}

export function useEventsByEntity(entityId: string) {
  const { asOfIso, isHistorical } = useTimeTravelOptional();

  // Include as_of in the SWR key
  const swrKey = entityId
    ? asOfIso
      ? `/api/events/entity/${entityId}?as_of=${asOfIso}`
      : `/api/events/entity/${entityId}`
    : null;

  const { data, error, isLoading } = useSWR(
    swrKey,
    async () => {
      // Set asOf on client before request
      apiClient.setAsOf(asOfIso);
      const response = await apiClient.getEventsByEntity(entityId);
      if (response.error) throw new Error(response.error.message);
      return response.data;
    },
    {
      revalidateOnFocus: false,
    }
  );

  return {
    events: data?.data || [],
    total: data?.count || 0,
    isLoading,
    isHistorical,
    error: error?.message,
  };
}
