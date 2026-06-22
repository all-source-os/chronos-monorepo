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

  // apiClient.request() already unwrapped the `{ data: [...] }` envelope, so
  // `data` is the ReplayProgress[] array, not the wrapper — `data.data` was
  // undefined, rendering the replay list empty despite rows.
  const raw: unknown = data;
  const list: ReplayProgress[] = (
    Array.isArray(raw) ? raw : ((raw as { data?: ReplayProgress[] })?.data ?? [])
  ) as ReplayProgress[];

  return {
    replays: list,
    total: Array.isArray(raw) ? raw.length : ((raw as { total?: number })?.total ?? list.length),
    isLoading,
    error: error?.message,
    startReplay,
    cancelReplay,
    deleteReplay,
    refresh: mutate,
  };
}
