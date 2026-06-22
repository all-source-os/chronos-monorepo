"use client";

import useSWR, { mutate } from "swr";
import { useTimeTravelOptional } from "@/hooks/use-time-travel";
import {
  apiClient,
  type CreateEventRequest,
  type Event,
  type EventListResponse,
  type ListEventsParams,
} from "@/lib/api/client";

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

  // apiClient.request() unwraps `{ data: X }` → X. The events endpoint returns
  // `{ count, data: [...] }`, so that unwrap yields the Event[] array directly —
  // the value here is already the array, NOT the EventListResponse wrapper.
  // Doing `data.data` again returned undefined, which silently rendered every
  // events/memory view empty even when the API returned rows. Handle both shapes.
  const raw = data as Event[] | EventListResponse | undefined;
  const list: Event[] = Array.isArray(raw) ? raw : (raw?.data ?? []);

  return {
    events: list,
    total: Array.isArray(raw) ? raw.length : (raw?.count ?? list.length),
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

  // Same unwrap caveat as useEvents: apiClient.request() already stripped the
  // `{ data: ... }` envelope, so `data` is the Event[] array — `data.data` is
  // undefined and silently rendered the entity view empty. Handle both shapes.
  const raw = data as Event[] | EventListResponse | undefined;
  const list: Event[] = Array.isArray(raw) ? raw : (raw?.data ?? []);

  return {
    events: list,
    total: Array.isArray(raw) ? raw.length : (raw?.count ?? list.length),
    isLoading,
    isHistorical,
    error: error?.message,
  };
}
