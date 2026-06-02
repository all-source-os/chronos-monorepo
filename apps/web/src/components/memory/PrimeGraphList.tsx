"use client";

import { Badge, Card, CardContent, cn, Input } from "@allsource/ui";
import { ChevronDown, ChevronRight, Compass, Search } from "lucide-react";
import { useMemo, useState } from "react";
import type { PrimeGraphEdge, PrimeGraphNode } from "@/lib/api/client";
import { colorForType, nodeLabel } from "./node-colors";

type NodeSort = "type" | "name" | "created";
type EdgeSort = "relation" | "source" | "created";

export interface PrimeGraphListProps {
  nodes: PrimeGraphNode[];
  edges: PrimeGraphEdge[];
  nodesByType: Record<string, number>;
}

export function PrimeGraphList({ nodes, edges, nodesByType }: PrimeGraphListProps) {
  const [tab, setTab] = useState<"nodes" | "edges">("nodes");
  const [search, setSearch] = useState("");
  const [typeFilter, setTypeFilter] = useState<string | null>(null);

  const nodeById = useMemo(() => {
    const m = new Map<string, PrimeGraphNode>();
    for (const n of nodes) m.set(n.id, n);
    return m;
  }, [nodes]);

  const labelFor = (id: string) => {
    const n = nodeById.get(id);
    return n ? nodeLabel(n.properties, id) : id.split(":").pop() || id;
  };

  const types = useMemo(
    () => Object.entries(nodesByType).sort((a, b) => b[1] - a[1]),
    [nodesByType]
  );

  return (
    <Card>
      <CardContent className="pt-6">
        {/* Sub-tabs + search */}
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <div className="inline-flex rounded-md border p-0.5">
            {(["nodes", "edges"] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => setTab(t)}
                className={cn(
                  "rounded px-3 py-1 text-xs font-medium capitalize transition-colors",
                  tab === t
                    ? "bg-muted text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {t} ({t === "nodes" ? nodes.length : edges.length})
              </button>
            ))}
          </div>
          <div className="relative ml-auto min-w-[200px] flex-1 sm:max-w-xs">
            <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={tab === "nodes" ? "Search nodes…" : "Search edges…"}
              className="h-8 pl-8 text-sm"
            />
          </div>
        </div>

        {tab === "nodes" && (
          <div className="mb-3 flex flex-wrap gap-1.5">
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
          </div>
        )}

        {tab === "nodes" ? (
          <NodeTable nodes={nodes} search={search} typeFilter={typeFilter} />
        ) : (
          <EdgeTable edges={edges} search={search} labelFor={labelFor} />
        )}
      </CardContent>
    </Card>
  );
}

function NodeTable({
  nodes,
  search,
  typeFilter,
}: {
  nodes: PrimeGraphNode[];
  search: string;
  typeFilter: string | null;
}) {
  const [sort, setSort] = useState<NodeSort>("type");
  const [asc, setAsc] = useState(true);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const searchLc = search.trim().toLowerCase();

  const rows = useMemo(() => {
    let list = nodes;
    if (typeFilter) list = list.filter((n) => n.node_type === typeFilter);
    if (searchLc) {
      list = list.filter((n) =>
        `${n.id} ${n.node_type} ${JSON.stringify(n.properties)}`.toLowerCase().includes(searchLc)
      );
    }
    const sorted = [...list].sort((a, b) => {
      let cmp = 0;
      if (sort === "type") cmp = a.node_type.localeCompare(b.node_type);
      else if (sort === "name")
        cmp = nodeLabel(a.properties, a.id).localeCompare(nodeLabel(b.properties, b.id));
      else cmp = a.created_at.localeCompare(b.created_at);
      return asc ? cmp : -cmp;
    });
    return sorted;
  }, [nodes, typeFilter, searchLc, sort, asc]);

  const toggle = (id: string) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const sortBy = (key: NodeSort) => {
    if (sort === key) setAsc((v) => !v);
    else {
      setSort(key);
      setAsc(true);
    }
  };

  const head = (key: NodeSort, label: string) => (
    <button
      type="button"
      onClick={() => sortBy(key)}
      className="flex items-center gap-1 font-medium hover:text-foreground"
    >
      {label}
      {sort === key && <span>{asc ? "▲" : "▼"}</span>}
    </button>
  );

  return (
    <div className="overflow-x-auto rounded-md border">
      <table className="w-full text-left text-xs">
        <thead className="border-b bg-muted/30 text-muted-foreground">
          <tr>
            <th className="w-6 px-2 py-2" />
            <th className="px-2 py-2">{head("type", "Type")}</th>
            <th className="px-2 py-2">{head("name", "Name / key properties")}</th>
            <th className="px-2 py-2">Vector</th>
            <th className="px-2 py-2">{head("created", "Created")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 && (
            <tr>
              <td colSpan={5} className="px-2 py-6 text-center text-muted-foreground">
                No nodes match.
              </td>
            </tr>
          )}
          {rows.map((n) => {
            const isOpen = expanded.has(n.id);
            return <NodeRow key={n.id} node={n} isOpen={isOpen} onToggle={() => toggle(n.id)} />;
          })}
        </tbody>
      </table>
    </div>
  );
}

function NodeRow({
  node,
  isOpen,
  onToggle,
}: {
  node: PrimeGraphNode;
  isOpen: boolean;
  onToggle: () => void;
}) {
  const keyProps = Object.entries(node.properties)
    .slice(0, 3)
    .map(([k, v]) => `${k}: ${typeof v === "string" ? v : JSON.stringify(v)}`)
    .join(" · ");

  return (
    <>
      <tr className="cursor-pointer border-b last:border-0 hover:bg-muted/20" onClick={onToggle}>
        <td className="px-2 py-2 align-top text-muted-foreground">
          {isOpen ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
        </td>
        <td className="px-2 py-2 align-top">
          <span className="inline-flex items-center gap-1.5">
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-full"
              style={{ backgroundColor: colorForType(node.node_type) }}
            />
            <span className="font-mono text-foreground">{node.node_type}</span>
          </span>
        </td>
        <td className="px-2 py-2 align-top">
          <div className="font-medium text-foreground">{nodeLabel(node.properties, node.id)}</div>
          {keyProps && <div className="truncate text-muted-foreground">{keyProps}</div>}
        </td>
        <td className="px-2 py-2 align-top">
          {node.has_vector ? (
            <Badge variant="outline" className="gap-1 font-mono text-[9px] text-cyan-400">
              <Compass className="h-3 w-3" />
              {node.vector_dim ?? "✓"}
            </Badge>
          ) : (
            <span className="text-muted-foreground">—</span>
          )}
        </td>
        <td className="whitespace-nowrap px-2 py-2 align-top font-mono text-muted-foreground">
          {node.created_at.slice(0, 19).replace("T", " ")}
        </td>
      </tr>
      {isOpen && (
        <tr className="border-b bg-muted/10 last:border-0">
          <td />
          <td colSpan={4} className="px-2 py-2">
            <code className="mb-2 block break-all font-mono text-[10px] text-muted-foreground">
              {node.id}
            </code>
            <pre className="overflow-x-auto rounded border bg-background/60 p-2 text-[11px] leading-relaxed">
              {JSON.stringify(node, null, 2)}
            </pre>
          </td>
        </tr>
      )}
    </>
  );
}

function EdgeTable({
  edges,
  search,
  labelFor,
}: {
  edges: PrimeGraphEdge[];
  search: string;
  labelFor: (id: string) => string;
}) {
  const [sort, setSort] = useState<EdgeSort>("relation");
  const [asc, setAsc] = useState(true);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  const searchLc = search.trim().toLowerCase();

  const rows = useMemo(() => {
    let list = edges.map((e, i) => ({ edge: e, i }));
    if (searchLc) {
      list = list.filter(({ edge }) =>
        `${edge.source} ${edge.target} ${edge.relation} ${JSON.stringify(edge.properties)}`
          .toLowerCase()
          .includes(searchLc)
      );
    }
    list.sort((a, b) => {
      let cmp = 0;
      if (sort === "relation") cmp = a.edge.relation.localeCompare(b.edge.relation);
      else if (sort === "source") cmp = a.edge.source.localeCompare(b.edge.source);
      else cmp = a.edge.created_at.localeCompare(b.edge.created_at);
      return asc ? cmp : -cmp;
    });
    return list;
  }, [edges, searchLc, sort, asc]);

  const toggle = (i: number) =>
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });

  const sortBy = (key: EdgeSort) => {
    if (sort === key) setAsc((v) => !v);
    else {
      setSort(key);
      setAsc(true);
    }
  };

  const head = (key: EdgeSort, label: string) => (
    <button
      type="button"
      onClick={() => sortBy(key)}
      className="flex items-center gap-1 font-medium hover:text-foreground"
    >
      {label}
      {sort === key && <span>{asc ? "▲" : "▼"}</span>}
    </button>
  );

  return (
    <div className="overflow-x-auto rounded-md border">
      <table className="w-full text-left text-xs">
        <thead className="border-b bg-muted/30 text-muted-foreground">
          <tr>
            <th className="w-6 px-2 py-2" />
            <th className="px-2 py-2">{head("source", "Source")}</th>
            <th className="px-2 py-2">{head("relation", "Relation")}</th>
            <th className="px-2 py-2">Target</th>
            <th className="px-2 py-2">Weight</th>
            <th className="px-2 py-2">{head("created", "Created")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 && (
            <tr>
              <td colSpan={6} className="px-2 py-6 text-center text-muted-foreground">
                No edges match.
              </td>
            </tr>
          )}
          {rows.map(({ edge, i }) => {
            const isOpen = expanded.has(i);
            return (
              <EdgeRow
                key={`${edge.source}-${edge.relation}-${edge.target}-${i}`}
                edge={edge}
                labelFor={labelFor}
                isOpen={isOpen}
                onToggle={() => toggle(i)}
              />
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function EdgeRow({
  edge,
  labelFor,
  isOpen,
  onToggle,
}: {
  edge: PrimeGraphEdge;
  labelFor: (id: string) => string;
  isOpen: boolean;
  onToggle: () => void;
}) {
  return (
    <>
      <tr className="cursor-pointer border-b last:border-0 hover:bg-muted/20" onClick={onToggle}>
        <td className="px-2 py-2 align-top text-muted-foreground">
          {isOpen ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
        </td>
        <td className="px-2 py-2 align-top text-foreground">{labelFor(edge.source)}</td>
        <td className="px-2 py-2 align-top">
          <Badge variant="outline" className="font-mono text-[9px]">
            {edge.relation}
          </Badge>
        </td>
        <td className="px-2 py-2 align-top text-foreground">{labelFor(edge.target)}</td>
        <td className="px-2 py-2 align-top font-mono text-muted-foreground">
          {edge.weight ?? "—"}
        </td>
        <td className="whitespace-nowrap px-2 py-2 align-top font-mono text-muted-foreground">
          {edge.created_at.slice(0, 19).replace("T", " ")}
        </td>
      </tr>
      {isOpen && (
        <tr className="border-b bg-muted/10 last:border-0">
          <td />
          <td colSpan={5} className="px-2 py-2">
            <div className="mb-2 break-all font-mono text-[10px] text-muted-foreground">
              {edge.source} → {edge.target}
            </div>
            <pre className="overflow-x-auto rounded border bg-background/60 p-2 text-[11px] leading-relaxed">
              {JSON.stringify(edge, null, 2)}
            </pre>
          </td>
        </tr>
      )}
    </>
  );
}
