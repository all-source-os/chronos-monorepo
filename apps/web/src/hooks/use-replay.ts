import useSWR from "swr";

import { apiClient, type ReplayProgress, type StartReplayRequest } from "@/lib/api/client";

export function useReplays() {
  const { data, error, isLoading, mutate } = useSWR(
    "/api/replay",
    async () => {
      const response = await apiClient.listReplays();
      if (response.error) throw new Error(response.error.message);
      return response.data;
    },
    { revalidateOnFocus: false, refreshInterval: 5000 }
  );

  const startReplay = async (params: StartReplayRequest): Promise<ReplayProgress | null> => {
    const response = await apiClient.startReplay(params);
    if (response.error) throw new Error(response.error.message);
    await mutate();
    return response.data ?? null;
  };

  const cancelReplay = async (replayId: string) => {
    const response = await apiClient.cancelReplay(replayId);
    if (response.error) throw new Error(response.error.message);
    await mutate();
  };

  const deleteReplay = async (replayId: string) => {
    const response = await apiClient.deleteReplay(replayId);
    if (response.error) throw new Error(response.error.message);
    await mutate();
  };

  return {
    replays: data?.data ?? [],
    total: data?.total ?? 0,
    isLoading,
    error: error?.message,
    startReplay,
    cancelReplay,
    deleteReplay,
    refresh: mutate,
  };
}
