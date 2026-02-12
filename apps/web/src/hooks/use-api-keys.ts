"use client";

import useSWR, { mutate } from "swr";
import { apiClient, type CreateApiKeyRequest, type UpdateApiKeyRequest } from "@/lib/api/client";

const fetcher = async () => {
  const response = await apiClient.listApiKeys();
  if (response.error) throw new Error(response.error.message);
  return response.data;
};

export function useApiKeys() {
  const { data, error, isLoading } = useSWR("/api/api-keys", fetcher, {
    revalidateOnFocus: false,
  });

  const createKey = async (data: CreateApiKeyRequest) => {
    const response = await apiClient.createApiKey(data);
    if (response.error) throw new Error(response.error.message);
    mutate("/api/api-keys");
    return response.data;
  };

  const updateKey = async (id: string, data: UpdateApiKeyRequest) => {
    const response = await apiClient.updateApiKey(id, data);
    if (response.error) throw new Error(response.error.message);
    mutate("/api/api-keys");
    return response.data;
  };

  const rotateKey = async (id: string) => {
    const response = await apiClient.rotateApiKey(id);
    if (response.error) throw new Error(response.error.message);
    mutate("/api/api-keys");
    return response.data;
  };

  const revokeKey = async (id: string) => {
    const response = await apiClient.revokeApiKey(id);
    if (response.error) throw new Error(response.error.message);
    mutate("/api/api-keys");
  };

  return {
    keys: data || [],
    isLoading,
    error: error?.message,
    createKey,
    updateKey,
    rotateKey,
    revokeKey,
    refresh: () => mutate("/api/api-keys"),
  };
}
