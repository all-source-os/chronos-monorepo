"use client";

import useSWR from "swr";
import {
  apiClient,
  type PrimeGraph,
  type PrimeGraphEdge,
  type PrimeGraphNode,
  type PrimeGraphParams,
  type PrimeGraphStats,
} from "@/lib/api/client";

const EMPTY_STATS: PrimeGraphStats = {
  node_count: 0,
  edge_count: 0,
  vector_count: 0,
  nodes_by_type: {},
};

const fetcher = async (key: string): Promise<PrimeGraph> => {
  const params: PrimeGraphParams = {};
  const searchParams = new URLSearchParams(key.split("?")[1] || "");
  if (searchParams.get("node_type")) params.node_type = searchParams.get("node_type")!;
  if (searchParams.get("limit")) params.limit = parseInt(searchParams.get("limit")!, 10);

  const response = await apiClient.getPrimeGraph(params);
  if (response.error) throw new Error(response.error.message);
  // apiClient.request() already does `data.data ?? data`. GET /api/v1/prime/graph
  // returns the graph object at the TOP level ({ nodes, edges, stats, has_more }) —
  // there is no `{ data: ... }` envelope — so response.data IS the PrimeGraph.
  // Do NOT reach for `.data` a second time here; that returned undefined for the
  // events hook and rendered every populated view empty. See use-events.ts.
  return response.data as PrimeGraph;
};

/**
 * Fetch the tenant's AllSource Prime knowledge graph.
 *
 * `node_type` and `limit` are passed through to the server (applied before the
 * limit is reached). Text/property search and most filtering is done
 * client-side over the returned set — see PrimeGraph.tsx / PrimeGraphList.tsx.
 */
export function useGraph(params?: PrimeGraphParams) {
  const queryParams: Record<string, string> = {};
  if (params?.node_type) queryParams.node_type = params.node_type;
  if (params?.limit !== undefined) queryParams.limit = String(params.limit);

  const queryString =
    Object.keys(queryParams).length > 0 ? `?${new URLSearchParams(queryParams).toString()}` : "";

  const { data, error, isLoading, isValidating, mutate } = useSWR(
    `/api/prime/graph${queryString}`,
    fetcher,
    {
      revalidateOnFocus: false,
      dedupingInterval: 5000,
    }
  );

  const nodes: PrimeGraphNode[] = data?.nodes ?? [];
  const edges: PrimeGraphEdge[] = data?.edges ?? [];
  const stats: PrimeGraphStats = data?.stats ?? EMPTY_STATS;
  const hasMore = data?.has_more ?? false;

  return {
    nodes,
    edges,
    stats,
    hasMore,
    isLoading,
    isValidating,
    error: error?.message as string | undefined,
    refresh: () => mutate(),
  };
}
