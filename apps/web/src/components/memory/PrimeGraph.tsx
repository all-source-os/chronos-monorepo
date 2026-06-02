"use client";

import { Badge, Card, CardContent, cn, Input } from "@allsource/ui";
import { Compass, Search, X } from "lucide-react";
import dynamic from "next/dynamic";
import { useCallback, useMemo, useRef, useState } from "react";
import type { PrimeGraphEdge, PrimeGraphNode } from "@/lib/api/client";
import { colorForType, nodeLabel } from "./node-colors";

// react-force-graph-2d paints to a <canvas> and touches `window` at module
// scope, so it MUST be client-only — a static import crashes the Next build
// during SSR. Dynamic import with ssr:false keeps it off the server.
const ForceGraph2D = dynamic(() => import("react-force-graph-2d"), {
  ssr: false,
  loading: () => (
    <div className="flex h-[560px] items-center justify-center text-sm text-muted-foreground">
      Loading graph…
    </div>
  ),
});

// react-force-graph mutates link.source/target into node refs at runtime.
interface ForceNode extends PrimeGraphNode {
  degree: number;
  color: string;
  label: string;
  // Populated by the force simulation at runtime.
  x?: number;
  y?: number;
}
interface ForceLink {
  source: string | ForceNode;
  target: string | ForceNode;
  relation: string;
  edge: PrimeGraphEdge;
}

export interface PrimeGraphProps {
  nodes: PrimeGraphNode[];
  edges: PrimeGraphEdge[];
  nodesByType: Record<string, number>;
}

export function PrimeGraph({ nodes, edges, nodesByType }: PrimeGraphProps) {
  const [selectedNode, setSelectedNode] = useState<ForceNode | null>(null);
  const [selectedEdge, setSelectedEdge] = useState<PrimeGraphEdge | null>(null);
  const [search, setSearch] = useState("");
  const [typeFilter, setTypeFilter] = useState<string | null>(null);
  const fgRef = useRef<unknown>(null);

  // Degree map (in + out) drives bubble size.
  const degree = useMemo(() => {
    const d = new Map<string, number>();
    for (const e of edges) {
      d.set(e.source, (d.get(e.source) ?? 0) + 1);
      d.set(e.target, (d.get(e.target) ?? 0) + 1);
    }
    return d;
  }, [edges]);

  const searchLc = search.trim().toLowerCase();

  // Which nodes pass the client-side text + type filter.
  const matchedIds = useMemo(() => {
    const set = new Set<string>();
    for (const n of nodes) {
      if (typeFilter && n.node_type !== typeFilter) continue;
      if (searchLc) {
        const hay = `${n.id} ${n.node_type} ${JSON.stringify(n.properties)}`.toLowerCase();
        if (!hay.includes(searchLc)) continue;
      }
      set.add(n.id);
    }
    return set;
  }, [nodes, typeFilter, searchLc]);

  const graphData = useMemo(() => {
    const fNodes: ForceNode[] = nodes.map((n) => ({
      ...n,
      degree: degree.get(n.id) ?? 0,
      color: colorForType(n.node_type),
      label: nodeLabel(n.properties, n.id),
    }));
    const present = new Set(fNodes.map((n) => n.id));
    const fLinks: ForceLink[] = edges
      .filter((e) => present.has(e.source) && present.has(e.target))
      .map((e) => ({
        source: e.source,
        target: e.target,
        relation: e.relation,
        edge: e,
      }));
    return { nodes: fNodes, links: fLinks };
  }, [nodes, edges, degree]);

  const edgesForNode = useCallback(
    (nodeId: string) => {
      const incoming = edges.filter((e) => e.target === nodeId);
      const outgoing = edges.filter((e) => e.source === nodeId);
      return { incoming, outgoing };
    },
    [edges]
  );

  const nodeById = useMemo(() => {
    const m = new Map<string, PrimeGraphNode>();
    for (const n of nodes) m.set(n.id, n);
    return m;
  }, [nodes]);

  const types = useMemo(
    () => Object.entries(nodesByType).sort((a, b) => b[1] - a[1]),
    [nodesByType]
  );

  const isFiltering = Boolean(typeFilter || searchLc);

  const paintNode = useCallback(
    (node: ForceNode, ctx: CanvasRenderingContext2D, globalScale: number) => {
      const radius = 4 + Math.sqrt(node.degree) * 2;
      const dimmed = isFiltering && !matchedIds.has(node.id);
      ctx.globalAlpha = dimmed ? 0.12 : 1;

      // has_vector ring
      if (node.has_vector) {
        ctx.beginPath();
        ctx.arc(node.x!, node.y!, radius + 2.5, 0, 2 * Math.PI);
        ctx.strokeStyle = "#22d3ee";
        ctx.lineWidth = 1 / globalScale;
        ctx.stroke();
      }

      ctx.beginPath();
      ctx.arc(node.x!, node.y!, radius, 0, 2 * Math.PI);
      ctx.fillStyle = node.color;
      ctx.fill();

      if (selectedNode?.id === node.id) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 2 / globalScale;
        ctx.stroke();
      }

      // Label only when zoomed in enough, to avoid clutter.
      if (globalScale > 1.5 && !dimmed) {
        const fontSize = 10 / globalScale;
        ctx.font = `${fontSize}px ui-sans-serif, system-ui`;
        ctx.fillStyle = "rgba(229,231,235,0.85)";
        ctx.textAlign = "center";
        ctx.textBaseline = "top";
        const text = node.label.length > 24 ? `${node.label.slice(0, 24)}…` : node.label;
        ctx.fillText(text, node.x!, node.y! + radius + 1);
      }
      ctx.globalAlpha = 1;
    },
    [isFiltering, matchedIds, selectedNode]
  );

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
      <Card className="overflow-hidden">
        <CardContent className="p-0">
          {/* Toolbar */}
          <div className="flex flex-wrap items-center gap-2 border-b p-3">
            <div className="relative flex-1 min-w-[180px]">
              <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Search nodes by name or property…"
                className="h-8 pl-8 text-sm"
              />
            </div>
            {typeFilter && (
              <button
                type="button"
                onClick={() => setTypeFilter(null)}
                className="inline-flex h-8 items-center gap-1 rounded-md border px-2 text-xs text-muted-foreground hover:text-foreground"
              >
                <X className="h-3 w-3" />
                {typeFilter}
              </button>
            )}
          </div>

          {/* Legend */}
          <div className="flex flex-wrap gap-1.5 border-b px-3 py-2">
            {types.map(([type, count]) => (
              <button
                key={type}
                type="button"
                onClick={() => setTypeFilter(typeFilter === type ? null : type)}
                className={cn(
                  "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] transition-colors",
                  typeFilter === type
                    ? "border-foreground/40 bg-muted"
                    : "border-transparent hover:bg-muted/50"
                )}
              >
                <span
                  className="h-2.5 w-2.5 rounded-full"
                  style={{ backgroundColor: colorForType(type) }}
                />
                <span className="font-mono text-foreground">{type}</span>
                <span className="text-muted-foreground">{count}</span>
              </button>
            ))}
            <span className="ml-auto inline-flex items-center gap-1 text-[11px] text-muted-foreground">
              <span className="h-2.5 w-2.5 rounded-full ring-1 ring-cyan-400" />
              has vector
            </span>
          </div>

          {/* Graph canvas */}
          <div className="h-[560px] w-full bg-[#0b0e14]">
            <ForceGraph2D
              ref={fgRef as never}
              graphData={graphData}
              nodeRelSize={1}
              nodeCanvasObject={paintNode as never}
              nodePointerAreaPaint={
                ((node: ForceNode, color: string, ctx: CanvasRenderingContext2D) => {
                  const radius = 4 + Math.sqrt(node.degree) * 2;
                  ctx.fillStyle = color;
                  ctx.beginPath();
                  ctx.arc(node.x!, node.y!, radius + 2, 0, 2 * Math.PI);
                  ctx.fill();
                }) as never
              }
              linkColor={
                (() => (isFiltering ? "rgba(148,163,184,0.12)" : "rgba(148,163,184,0.25)")) as never
              }
              linkWidth={0.6}
              linkDirectionalArrowLength={2.5}
              linkDirectionalArrowRelPos={1}
              linkLabel={((l: ForceLink) => l.relation) as never}
              onNodeClick={
                ((node: ForceNode) => {
                  setSelectedEdge(null);
                  setSelectedNode(node);
                }) as never
              }
              onLinkClick={
                ((link: ForceLink) => {
                  setSelectedNode(null);
                  setSelectedEdge(link.edge);
                }) as never
              }
              onBackgroundClick={() => {
                setSelectedNode(null);
                setSelectedEdge(null);
              }}
              cooldownTicks={100}
            />
          </div>
        </CardContent>
      </Card>

      {/* Detail panel */}
      <Card className="h-fit lg:sticky lg:top-4">
        <CardContent className="pt-6">
          {!selectedNode && !selectedEdge && (
            <div className="text-sm text-muted-foreground">
              <p className="mb-2 font-medium text-foreground">Inspect the graph</p>
              <p>
                Click a bubble to see a node&apos;s full stored properties and its edges, or click
                an edge to see its relation and properties. Click a legend chip to isolate a type;
                search filters by name or any stored property.
              </p>
            </div>
          )}

          {selectedNode && (
            <NodeDetail
              node={selectedNode}
              edges={edgesForNode(selectedNode.id)}
              nodeById={nodeById}
              onPickNeighbor={(id) => {
                const n = graphData.nodes.find((x) => x.id === id);
                if (n) {
                  setSelectedEdge(null);
                  setSelectedNode(n);
                }
              }}
            />
          )}

          {selectedEdge && <EdgeDetail edge={selectedEdge} nodeById={nodeById} />}
        </CardContent>
      </Card>
    </div>
  );
}

function NodeDetail({
  node,
  edges,
  nodeById,
  onPickNeighbor,
}: {
  node: ForceNode;
  edges: { incoming: PrimeGraphEdge[]; outgoing: PrimeGraphEdge[] };
  nodeById: Map<string, PrimeGraphNode>;
  onPickNeighbor: (id: string) => void;
}) {
  const neighborLabel = (id: string) => {
    const n = nodeById.get(id);
    return n ? nodeLabel(n.properties, id) : id.split(":").pop() || id;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <span className="h-3 w-3 rounded-full" style={{ backgroundColor: node.color }} />
        <Badge variant="outline" className="font-mono text-[10px]">
          {node.node_type}
        </Badge>
        {node.has_vector && (
          <Badge variant="outline" className="gap-1 font-mono text-[10px] text-cyan-400">
            <Compass className="h-3 w-3" />
            vector{node.vector_dim ? ` · ${node.vector_dim}d` : ""}
          </Badge>
        )}
      </div>
      <div>
        <div className="text-sm font-semibold text-foreground">{node.label}</div>
        <code className="break-all font-mono text-[10px] text-muted-foreground">{node.id}</code>
      </div>

      <PropertyTable properties={node.properties} />

      <div className="grid grid-cols-2 gap-2 text-[11px] text-muted-foreground">
        <div>
          <span className="block uppercase tracking-wide">Created</span>
          <span className="font-mono">{node.created_at}</span>
        </div>
        <div>
          <span className="block uppercase tracking-wide">Updated</span>
          <span className="font-mono">{node.updated_at}</span>
        </div>
      </div>

      <EdgeGroup
        title={`Outgoing (${edges.outgoing.length})`}
        rows={edges.outgoing.map((e) => ({ rel: e.relation, other: e.target }))}
        neighborLabel={neighborLabel}
        onPick={onPickNeighbor}
      />
      <EdgeGroup
        title={`Incoming (${edges.incoming.length})`}
        rows={edges.incoming.map((e) => ({ rel: e.relation, other: e.source }))}
        neighborLabel={neighborLabel}
        onPick={onPickNeighbor}
      />
    </div>
  );
}

function EdgeGroup({
  title,
  rows,
  neighborLabel,
  onPick,
}: {
  title: string;
  rows: { rel: string; other: string }[];
  neighborLabel: (id: string) => string;
  onPick: (id: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <div>
      <div className="mb-1.5 text-xs font-medium text-foreground">{title}</div>
      <div className="space-y-1">
        {rows.map((r) => (
          <button
            key={`${r.rel}-${r.other}`}
            type="button"
            onClick={() => onPick(r.other)}
            className="flex w-full items-center gap-2 rounded border bg-muted/10 px-2 py-1 text-left text-xs hover:bg-muted/30"
          >
            <Badge variant="outline" className="shrink-0 font-mono text-[9px]">
              {r.rel}
            </Badge>
            <span className="truncate text-muted-foreground">{neighborLabel(r.other)}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function EdgeDetail({
  edge,
  nodeById,
}: {
  edge: PrimeGraphEdge;
  nodeById: Map<string, PrimeGraphNode>;
}) {
  const label = (id: string) => {
    const n = nodeById.get(id);
    return n ? nodeLabel(n.properties, id) : id.split(":").pop() || id;
  };
  return (
    <div className="space-y-4">
      <Badge variant="outline" className="font-mono text-[10px]">
        edge
      </Badge>
      <div className="flex items-center gap-2 text-sm">
        <span className="truncate font-medium text-foreground">{label(edge.source)}</span>
        <Badge variant="outline" className="shrink-0 font-mono text-[9px]">
          {edge.relation}
        </Badge>
        <span className="truncate font-medium text-foreground">{label(edge.target)}</span>
      </div>
      {edge.weight != null && (
        <div className="text-xs text-muted-foreground">
          Weight: <span className="font-mono text-foreground">{edge.weight}</span>
        </div>
      )}
      <div className="text-[11px] text-muted-foreground">
        <span className="block uppercase tracking-wide">Created</span>
        <span className="font-mono">{edge.created_at}</span>
      </div>
      {edge.properties && Object.keys(edge.properties).length > 0 ? (
        <PropertyTable properties={edge.properties} />
      ) : (
        <p className="text-xs text-muted-foreground">No edge properties.</p>
      )}
    </div>
  );
}

function PropertyTable({ properties }: { properties: Record<string, unknown> }) {
  const entries = Object.entries(properties);
  if (entries.length === 0) {
    return <p className="text-xs text-muted-foreground">No stored properties.</p>;
  }
  return (
    <div className="rounded-md border">
      <table className="w-full text-xs">
        <tbody>
          {entries.map(([key, value]) => (
            <tr key={key} className="border-b last:border-0">
              <td className="w-1/3 whitespace-nowrap px-2 py-1.5 align-top font-mono text-muted-foreground">
                {key}
              </td>
              <td className="break-all px-2 py-1.5 align-top text-foreground">
                {typeof value === "string" ? value : JSON.stringify(value)}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
