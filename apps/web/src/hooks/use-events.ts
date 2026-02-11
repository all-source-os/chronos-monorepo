"use client";

import useSWR, { mutate } from "swr";
import { apiClient, type Event, type CreateEventRequest, type ListEventsParams } from "@/lib/api/client";

const fetcher = async (key: string) => {
  const params: ListEventsParams = {};
  const searchParams = new URLSearchParams(key.split("?")[1] || "");

  if (searchParams.get("entity_id")) params.entity_id = searchParams.get("entity_id")!;
  if (searchParams.get("event_type")) params.event_type = searchParams.get("event_type")!;
  if (searchParams.get("limit")) params.limit = parseInt(searchParams.get("limit")!, 10);
  if (searchParams.get("offset")) params.offset = parseInt(searchParams.get("offset")!, 10);

  const response = await apiClient.listEvents(params);
  if (response.error) throw new Error(response.error.message);
  return response.data;
};

export function useEvents(params?: ListEventsParams) {
  const queryString = params
    ? `?${new URLSearchParams(
        Object.entries(params).reduce(
          (acc, [key, value]) => {
            if (value !== undefined) acc[key] = String(value);
            return acc;
          },
          {} as Record<string, string>
        )
      ).toString()}`
    : "";

  const { data, error, isLoading, isValidating } = useSWR(
    `/api/events${queryString}`,
    fetcher,
    {
      revalidateOnFocus: false,
      dedupingInterval: 5000,
    }
  );

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
    error: error?.message,
    createEvent,
    createEventBatch,
    refresh: () => mutate(`/api/events${queryString}`),
  };
}

export function useEventsByEntity(entityId: string) {
  const { data, error, isLoading } = useSWR(
    entityId ? `/api/events/entity/${entityId}` : null,
    async () => {
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
    error: error?.message,
  };
}
