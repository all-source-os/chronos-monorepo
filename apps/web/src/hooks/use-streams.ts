"use client";

import useSWR from "swr";
import { apiClient, type StreamInfo } from "@/lib/api/client";

interface StreamsParams {
  limit?: number;
  offset?: number;
}

const fetcher = async (key: string) => {
  const params: StreamsParams = {};
  const searchParams = new URLSearchParams(key.split("?")[1] || "");

  if (searchParams.get("limit")) params.limit = parseInt(searchParams.get("limit")!, 10);
  if (searchParams.get("offset")) params.offset = parseInt(searchParams.get("offset")!, 10);

  const response = await apiClient.listStreams(params);
  if (response.error) throw new Error(response.error.message);
  return response.data;
};

export function useStreams(params?: StreamsParams) {
  const queryParams: Record<string, string> = {};
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) queryParams[key] = String(value);
    }
  }

  const queryString =
    Object.keys(queryParams).length > 0 ? `?${new URLSearchParams(queryParams).toString()}` : "";

  const { data, error, isLoading, isValidating, mutate } = useSWR(
    `/api/streams${queryString}`,
    fetcher,
    {
      revalidateOnFocus: false,
      dedupingInterval: 10000,
    }
  );

  return {
    streams: (data?.data || []) as StreamInfo[],
    total: data?.total || 0,
    isLoading,
    isValidating,
    error: error?.message,
    refresh: () => mutate(),
  };
}
