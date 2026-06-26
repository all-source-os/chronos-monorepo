"use client";

import { Badge, Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Brain,
  ChevronRight,
  GitBranch,
  Loader2,
  Network,
  Plus,
  Search,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { useCallback, useRef, useState } from "react";

// =============================================================================
// Types
// =============================================================================

interface PrimeNode {
  node_id: string;
  entity_id: string;
  type: string;
  properties: Record<string, unknown>;
}

interface PrimeEdge {
  edge_id: string;
  source: string;
  target: string;
  relation: string;
}

interface RecallResult {
  id: string;
  type: string;
  properties: Record<string, unknown>;
  score: number;
  depth: number;
}

interface PrimeStats {
  total_nodes: number;
  total_edges: number;
  event_count: number;
  nodes_by_type: Record<string, number>;
  edges_by_relation: Record<string, number>;
}

type PlaygroundTab = "build" | "query" | "stats";

const PRIME_API = "/api/v1/prime";

const NODE_TYPES = [
  "person",
  "concept",
  "project",
  "metric",
  "decision",
  "event",
  "insight",
];

const RELATION_TYPES = [
  "works_on",
  "knows",
  "impacts",
  "requires",
  "depends_on",
  "authored",
  "manages",
  "related_to",
];

const SAMPLE_GRAPHS = [
  {
    name: "Team Graph",
    description: "3 people working on a project",
    nodes: [
      { type: "person", properties: { name: "Alice", role: "engineer", domain: "engineering" } },
      { type: "person", properties: { name: "Bob", role: "designer", domain: "design" } },
      { type: "project", properties: { name: "Prime", status: "active", domain: "engineering" } },
    ],
    edges: [
      { sourceIdx: 0, targetIdx: 2, relation: "works_on" },
      { sourceIdx: 1, targetIdx: 2, relation: "works_on" },
      { sourceIdx: 0, targetIdx: 1, relation: "knows" },
    ],
  },
  {
    name: "Decision Chain",
    description: "Decisions with dependencies",
    nodes: [
      { type: "decision", properties: { name: "Use Rust", rationale: "Performance", domain: "engineering" } },
      { type: "decision", properties: { name: "Event Sourcing", rationale: "Audit trail", domain: "engineering" } },
      { type: "concept", properties: { name: "Durability", domain: "engineering" } },
      { type: "metric", properties: { name: "99.9% uptime", domain: "operations" } },
    ],
    edges: [
      { sourceIdx: 0, targetIdx: 1, relation: "depends_on" },
      { sourceIdx: 1, targetIdx: 2, relation: "requires" },
      { sourceIdx: 2, targetIdx: 3, relation: "impacts" },
    ],
  },
];

// =============================================================================
// API helpers
// =============================================================================

async function primePost<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${PRIME_API}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}

async function primeGet<T>(path: string): Promise<T> {
  const res = await fetch(`${PRIME_API}${path}`);
  return res.json();
}

async function primeDelete(path: string): Promise<void> {
  await fetch(`${PRIME_API}${path}`, { method: "DELETE" });
}

// =============================================================================
// Main Component
// =============================================================================

export function PrimePlayground() {
  const [activeTab, setActiveTab] = useState<PlaygroundTab>("build");
  const [nodes, setNodes] = useState<PrimeNode[]>([]);
  const [edges, setEdges] = useState<PrimeEdge[]>([]);
  const [stats, setStats] = useState<PrimeStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const refreshStats = useCallback(async () => {
    try {
      const data = await primeGet<PrimeStats>("/stats");
      setStats(data);
    } catch {
      // Silently fail — stats panel shows placeholder
    }
  }, []);

  const tabs: { id: PlaygroundTab; label: string; icon: React.ElementType }[] = [
    { id: "build", label: "Build Graph", icon: Network },
    { id: "query", label: "Query & Recall", icon: Search },
    { id: "stats", label: "Live Stats", icon: Sparkles },
  ];

  return (
    <Card className="flex h-full min-h-[600px] flex-col">
      {/* Header with tabs */}
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <Brain className="h-5 w-5 text-primary" />
          <h3 className="text-sm font-semibold">Prime Playground</h3>
        </div>
        <div className="flex items-center rounded-lg border border-border bg-muted/50 p-0.5">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => {
                setActiveTab(tab.id);
                if (tab.id === "stats") refreshStats();
              }}
              className={cn(
                "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
                activeTab === tab.id
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground"
              )}
            >
              <tab.icon className="h-3.5 w-3.5" />
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <CardContent className="flex-1 overflow-hidden p-4">
        {activeTab === "build" && (
          <BuildPanel
            nodes={nodes}
            edges={edges}
            setNodes={setNodes}
            setEdges={setEdges}
            loading={loading}
            setLoading={setLoading}
            message={message}
            setMessage={setMessage}
            refreshStats={refreshStats}
          />
        )}
        {activeTab === "query" && (
          <QueryPanel nodes={nodes} />
        )}
        {activeTab === "stats" && (
          <StatsPanel stats={stats} onRefresh={refreshStats} />
        )}
      </CardContent>
    </Card>
  );
}

// =============================================================================
// Build Panel — Create nodes and edges
// =============================================================================

interface BuildPanelProps {
  nodes: PrimeNode[];
  edges: PrimeEdge[];
  setNodes: React.Dispatch<React.SetStateAction<PrimeNode[]>>;
  setEdges: React.Dispatch<React.SetStateAction<PrimeEdge[]>>;
  loading: boolean;
  setLoading: React.Dispatch<React.SetStateAction<boolean>>;
  message: string | null;
  setMessage: React.Dispatch<React.SetStateAction<string | null>>;
  refreshStats: () => Promise<void>;
}

function BuildPanel({
  nodes,
  edges,
  setNodes,
  setEdges,
  loading,
  setLoading,
  message,
  setMessage,
  refreshStats,
}: BuildPanelProps) {
  const [nodeType, setNodeType] = useState("person");
  const [nodeName, setNodeName] = useState("");
  const [nodeDomain, setNodeDomain] = useState("");
  const [edgeSource, setEdgeSource] = useState("");
  const [edgeTarget, setEdgeTarget] = useState("");
  const [edgeRelation, setEdgeRelation] = useState("works_on");
  const nameInputRef = useRef<HTMLInputElement>(null);

  const addNode = useCallback(async () => {
    if (!nodeName.trim()) return;
    setLoading(true);
    setMessage(null);
    try {
      const properties = {
        name: nodeName.trim(),
        domain: nodeDomain.trim() || nodeType,
      };
      // POST /nodes returns only { node_id, entity_id } — it does NOT echo the
      // type/properties we sent. Build the local node from our input plus the
      // returned ids, or the list renders the UUID and the name is lost.
      const data = await primePost<{ node_id: string; entity_id: string }>("/nodes", {
        type: nodeType,
        properties,
      });
      setNodes((prev) => [
        ...prev,
        { node_id: data.node_id, entity_id: data.entity_id, type: nodeType, properties },
      ]);
      setNodeName("");
      setMessage(`Created ${nodeType} "${nodeName.trim()}"`);
      nameInputRef.current?.focus();
      await refreshStats();
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : "Failed"}`);
    } finally {
      setLoading(false);
    }
  }, [nodeType, nodeName, nodeDomain, setNodes, setLoading, setMessage, refreshStats]);

  const addEdge = useCallback(async () => {
    if (!edgeSource || !edgeTarget) return;
    setLoading(true);
    setMessage(null);
    try {
      // POST /edges returns only { edge_id } — not source/target/relation. Build
      // the local edge from our input + the returned id, else the edge list
      // renders "undefined > undefined".
      const data = await primePost<{ edge_id: string }>("/edges", {
        source: edgeSource,
        target: edgeTarget,
        relation: edgeRelation,
      });
      setEdges((prev) => [
        ...prev,
        { edge_id: data.edge_id, source: edgeSource, target: edgeTarget, relation: edgeRelation },
      ]);
      setMessage(`Created edge: ${edgeRelation}`);
      await refreshStats();
    } catch (err) {
      setMessage(`Error: ${err instanceof Error ? err.message : "Failed"}`);
    } finally {
      setLoading(false);
    }
  }, [edgeSource, edgeTarget, edgeRelation, setEdges, setLoading, setMessage, refreshStats]);

  const loadSample = useCallback(
    async (sample: (typeof SAMPLE_GRAPHS)[0]) => {
      setLoading(true);
      setMessage(`Loading "${sample.name}"...`);
      try {
        const createdNodes: PrimeNode[] = [];
        for (const nodeDef of sample.nodes) {
          // Create returns only ids; keep the type/properties we sent so the
          // node renders by name, not UUID.
          const data = await primePost<{ node_id: string; entity_id: string }>("/nodes", {
            type: nodeDef.type,
            properties: nodeDef.properties,
          });
          createdNodes.push({
            node_id: data.node_id,
            entity_id: data.entity_id,
            type: nodeDef.type,
            properties: nodeDef.properties,
          });
        }

        const createdEdges: PrimeEdge[] = [];
        for (const edgeDef of sample.edges) {
          const src = createdNodes[edgeDef.sourceIdx];
          const tgt = createdNodes[edgeDef.targetIdx];
          if (src && tgt) {
            // Create returns only edge_id; keep source/target/relation so the
            // edge resolves to node names instead of "undefined > undefined".
            const data = await primePost<{ edge_id: string }>("/edges", {
              source: src.entity_id,
              target: tgt.entity_id,
              relation: edgeDef.relation,
            });
            createdEdges.push({
              edge_id: data.edge_id,
              source: src.entity_id,
              target: tgt.entity_id,
              relation: edgeDef.relation,
            });
          }
        }

        setNodes((prev) => [...prev, ...createdNodes]);
        setEdges((prev) => [...prev, ...createdEdges]);
        setMessage(
          `Loaded "${sample.name}": ${createdNodes.length} nodes, ${createdEdges.length} edges`
        );
        await refreshStats();
      } catch (err) {
        setMessage(`Error: ${err instanceof Error ? err.message : "Failed"}`);
      } finally {
        setLoading(false);
      }
    },
    [setNodes, setEdges, setLoading, setMessage, refreshStats]
  );

  const deleteNode = useCallback(
    async (entityId: string) => {
      try {
        await primeDelete(`/nodes/${encodeURIComponent(entityId)}`);
        setNodes((prev) => prev.filter((n) => n.entity_id !== entityId));
        setEdges((prev) =>
          prev.filter((e) => e.source !== entityId && e.target !== entityId)
        );
        setMessage("Deleted node");
        await refreshStats();
      } catch {
        setMessage("Failed to delete");
      }
    },
    [setNodes, setEdges, setMessage, refreshStats]
  );

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto">
      {/* Quick load */}
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs font-medium text-muted-foreground">Quick start:</span>
        {SAMPLE_GRAPHS.map((sample) => (
          <Button
            key={sample.name}
            variant="outline"
            size="sm"
            onClick={() => loadSample(sample)}
            disabled={loading}
            className="h-7 text-xs"
          >
            <Plus className="mr-1 h-3 w-3" />
            {sample.name}
          </Button>
        ))}
      </div>

      {/* Add node form */}
      <div className="rounded-lg border border-border bg-muted/30 p-3">
        <h4 className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          Add Node
        </h4>
        <div className="flex flex-wrap items-end gap-2">
          <div className="flex-1 min-w-[120px]">
            <label className="mb-1 block text-xs text-muted-foreground">Type</label>
            <select
              value={nodeType}
              onChange={(e) => setNodeType(e.target.value)}
              className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm"
            >
              {NODE_TYPES.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
          </div>
          <div className="flex-[2] min-w-[150px]">
            <label className="mb-1 block text-xs text-muted-foreground">Name</label>
            <input
              ref={nameInputRef}
              type="text"
              value={nodeName}
              onChange={(e) => setNodeName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addNode()}
              placeholder="e.g. Alice"
              className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm placeholder:text-muted-foreground"
            />
          </div>
          <div className="flex-1 min-w-[120px]">
            <label className="mb-1 block text-xs text-muted-foreground">Domain</label>
            <input
              type="text"
              value={nodeDomain}
              onChange={(e) => setNodeDomain(e.target.value)}
              placeholder="auto"
              className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm placeholder:text-muted-foreground"
            />
          </div>
          <Button onClick={addNode} disabled={loading || !nodeName.trim()} size="sm">
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
          </Button>
        </div>
      </div>

      {/* Add edge form */}
      {nodes.length >= 2 && (
        <div className="rounded-lg border border-border bg-muted/30 p-3">
          <h4 className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Add Edge
          </h4>
          <div className="flex flex-wrap items-end gap-2">
            <div className="flex-1 min-w-[130px]">
              <label className="mb-1 block text-xs text-muted-foreground">Source</label>
              <select
                value={edgeSource}
                onChange={(e) => setEdgeSource(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm"
              >
                <option value="">Select...</option>
                {nodes.map((n) => (
                  <option key={n.entity_id} value={n.entity_id}>
                    {String(n.properties?.name || n.node_id)}
                  </option>
                ))}
              </select>
            </div>
            <div className="flex items-center px-1">
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex-1 min-w-[100px]">
              <label className="mb-1 block text-xs text-muted-foreground">Relation</label>
              <select
                value={edgeRelation}
                onChange={(e) => setEdgeRelation(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm"
              >
                {RELATION_TYPES.map((r) => (
                  <option key={r} value={r}>{r}</option>
                ))}
              </select>
            </div>
            <div className="flex items-center px-1">
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            </div>
            <div className="flex-1 min-w-[130px]">
              <label className="mb-1 block text-xs text-muted-foreground">Target</label>
              <select
                value={edgeTarget}
                onChange={(e) => setEdgeTarget(e.target.value)}
                className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-sm"
              >
                <option value="">Select...</option>
                {nodes.map((n) => (
                  <option key={n.entity_id} value={n.entity_id}>
                    {String(n.properties?.name || n.node_id)}
                  </option>
                ))}
              </select>
            </div>
            <Button
              onClick={addEdge}
              disabled={loading || !edgeSource || !edgeTarget}
              size="sm"
            >
              <GitBranch className="h-4 w-4" />
            </Button>
          </div>
        </div>
      )}

      {/* Status message */}
      {message && (
        <div
          className={cn(
            "flex items-center justify-between rounded-md px-3 py-2 text-xs",
            message.startsWith("Error")
              ? "bg-destructive/10 text-destructive"
              : "bg-primary/10 text-primary"
          )}
        >
          <span>{message}</span>
          <button onClick={() => setMessage(null)}>
            <X className="h-3 w-3" />
          </button>
        </div>
      )}

      {/* Node list */}
      {nodes.length > 0 && (
        <div className="space-y-1.5">
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Nodes ({nodes.length})
          </h4>
          <div className="space-y-1">
            {nodes.map((node) => (
              <div
                key={node.entity_id}
                className="flex items-center justify-between rounded-md border border-border bg-background px-3 py-2"
              >
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className="text-[10px]">
                    {node.type}
                  </Badge>
                  <span className="text-sm font-medium">
                    {String(node.properties?.name || node.node_id)}
                  </span>
                  <span className="text-[10px] text-muted-foreground font-mono">
                    {node.entity_id}
                  </span>
                </div>
                <button
                  onClick={() => deleteNode(node.entity_id)}
                  className="text-muted-foreground hover:text-destructive"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Edge list */}
      {edges.length > 0 && (
        <div className="space-y-1.5">
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Edges ({edges.length})
          </h4>
          <div className="space-y-1">
            {edges.map((edge) => {
              const srcNode = nodes.find((n) => n.entity_id === edge.source);
              const tgtNode = nodes.find((n) => n.entity_id === edge.target);
              return (
                <div
                  key={edge.edge_id}
                  className="flex items-center gap-2 rounded-md border border-border bg-background px-3 py-2 text-sm"
                >
                  <span className="font-medium">
                    {String(srcNode?.properties?.name || edge.source)}
                  </span>
                  <ChevronRight className="h-3 w-3 text-muted-foreground" />
                  <Badge variant="outline" className="text-[10px]">
                    {edge.relation}
                  </Badge>
                  <ChevronRight className="h-3 w-3 text-muted-foreground" />
                  <span className="font-medium">
                    {String(tgtNode?.properties?.name || edge.target)}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Query Panel — Neighbors, search, recall
// =============================================================================

function QueryPanel({ nodes }: { nodes: PrimeNode[] }) {
  const [queryType, setQueryType] = useState<"neighbors" | "search" | "recall">("neighbors");
  const [queryInput, setQueryInput] = useState("");
  const [results, setResults] = useState<RecallResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);

  const executeQuery = useCallback(async () => {
    if (!queryInput.trim()) return;
    setSearching(true);
    setResults([]);
    const start = performance.now();

    try {
      if (queryType === "neighbors") {
        const data = await primeGet<{ nodes: RecallResult[] }>(
          `/nodes/${encodeURIComponent(queryInput)}/neighbors`
        );
        setResults(
          (data.nodes || []).map((n) => ({ ...n, score: 1, depth: 1 }))
        );
      } else if (queryType === "search") {
        // Search nodes by type
        const data = await primeGet<{ nodes: RecallResult[] }>(
          `/nodes?type=${encodeURIComponent(queryInput)}`
        );
        setResults((data.nodes || []).map((n) => ({ ...n, score: 1, depth: 0 })));
      } else {
        // Recall — needs a vector, use stats endpoint as fallback
        const data = await primePost<{ nodes: RecallResult[] }>("/recall", {
          node_type: queryInput,
          top_k: 10,
        });
        setResults(data.nodes || []);
      }
    } catch {
      setResults([]);
    } finally {
      setLatencyMs(Math.round(performance.now() - start));
      setSearching(false);
    }
  }, [queryType, queryInput]);

  return (
    <div className="flex h-full flex-col gap-4">
      {/* Query type selector */}
      <div className="flex items-center gap-2">
        {(["neighbors", "search", "recall"] as const).map((qt) => (
          <button
            key={qt}
            onClick={() => setQueryType(qt)}
            className={cn(
              "rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
              queryType === qt
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:text-foreground"
            )}
          >
            {qt}
          </button>
        ))}
        {latencyMs !== null && (
          <Badge variant="secondary" className="ml-auto text-[10px]">
            {latencyMs}ms
          </Badge>
        )}
      </div>

      {/* Query input */}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          executeQuery();
        }}
        className="flex gap-2"
      >
        {queryType === "neighbors" && nodes.length > 0 ? (
          <select
            value={queryInput}
            onChange={(e) => setQueryInput(e.target.value)}
            className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm"
          >
            <option value="">Select a node...</option>
            {nodes.map((n) => (
              <option key={n.entity_id} value={n.entity_id}>
                {String(n.properties?.name || n.node_id)} ({n.type})
              </option>
            ))}
          </select>
        ) : (
          <input
            type="text"
            value={queryInput}
            onChange={(e) => setQueryInput(e.target.value)}
            placeholder={
              queryType === "search"
                ? "Node type (e.g. person)"
                : "Node type to recall"
            }
            className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm placeholder:text-muted-foreground"
          />
        )}
        <Button type="submit" disabled={searching || !queryInput.trim()} size="sm">
          {searching ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Search className="h-4 w-4" />
          )}
        </Button>
      </form>

      {/* Results */}
      <div className="flex-1 space-y-1.5 overflow-y-auto">
        {results.length === 0 && !searching && (
          <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
            <Search className="mb-2 h-8 w-8 opacity-30" />
            <p className="text-sm">
              {queryType === "neighbors"
                ? "Select a node to explore its neighbors"
                : "Enter a query to search"}
            </p>
          </div>
        )}
        {results.map((r, i) => (
          <div
            key={`${r.id}-${i}`}
            className="rounded-md border border-border bg-background p-3"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Badge variant="secondary" className="text-[10px]">
                  {r.type}
                </Badge>
                <span className="text-sm font-medium">
                  {String(r.properties?.name || r.id)}
                </span>
              </div>
              {r.score < 1 && (
                <Badge
                  variant="outline"
                  className={cn(
                    "text-[10px]",
                    r.score >= 0.7
                      ? "border-green-500/50 text-green-600"
                      : r.score >= 0.4
                        ? "border-yellow-500/50 text-yellow-600"
                        : "border-border text-muted-foreground"
                  )}
                >
                  {(r.score * 100).toFixed(0)}%
                </Badge>
              )}
            </div>
            {r.depth > 0 && (
              <span className="mt-1 block text-[10px] text-muted-foreground">
                depth: {r.depth}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// =============================================================================
// Stats Panel — Live graph statistics
// =============================================================================

function StatsPanel({
  stats,
  onRefresh,
}: {
  stats: PrimeStats | null;
  onRefresh: () => Promise<void>;
}) {
  const [refreshing, setRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await onRefresh();
    setRefreshing(false);
  }, [onRefresh]);

  if (!stats) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <Sparkles className="mb-3 h-10 w-10 text-muted-foreground/30" />
        <p className="mb-4 text-sm text-muted-foreground">
          No stats yet — add some nodes first
        </p>
        <Button onClick={handleRefresh} variant="outline" size="sm">
          {refreshing ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Sparkles className="mr-2 h-4 w-4" />
          )}
          Load Stats
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
          Graph Overview
        </h4>
        <Button onClick={handleRefresh} variant="ghost" size="sm" className="h-7">
          {refreshing ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Sparkles className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>

      {/* Metric cards */}
      <div className="grid grid-cols-3 gap-3">
        {[
          { label: "Nodes", value: stats.total_nodes },
          { label: "Edges", value: stats.total_edges },
          { label: "Events", value: stats.event_count },
        ].map((m) => (
          <div
            key={m.label}
            className="rounded-lg border border-border bg-muted/30 p-3 text-center"
          >
            <p className="text-2xl font-bold tracking-tight">{m.value}</p>
            <p className="text-[10px] text-muted-foreground uppercase">{m.label}</p>
          </div>
        ))}
      </div>

      {/* Node types */}
      {Object.keys(stats.nodes_by_type).length > 0 && (
        <div>
          <h4 className="mb-2 text-xs font-semibold text-muted-foreground">
            Nodes by Type
          </h4>
          <div className="space-y-1">
            {Object.entries(stats.nodes_by_type)
              .sort(([, a], [, b]) => b - a)
              .map(([type, count]) => (
                <div
                  key={type}
                  className="flex items-center justify-between rounded-md bg-muted/30 px-3 py-1.5"
                >
                  <Badge variant="secondary" className="text-[10px]">
                    {type}
                  </Badge>
                  <span className="text-sm font-mono">{count}</span>
                </div>
              ))}
          </div>
        </div>
      )}

      {/* Relation types */}
      {Object.keys(stats.edges_by_relation).length > 0 && (
        <div>
          <h4 className="mb-2 text-xs font-semibold text-muted-foreground">
            Edges by Relation
          </h4>
          <div className="space-y-1">
            {Object.entries(stats.edges_by_relation)
              .sort(([, a], [, b]) => b - a)
              .map(([relation, count]) => (
                <div
                  key={relation}
                  className="flex items-center justify-between rounded-md bg-muted/30 px-3 py-1.5"
                >
                  <Badge variant="outline" className="text-[10px]">
                    {relation}
                  </Badge>
                  <span className="text-sm font-mono">{count}</span>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}
