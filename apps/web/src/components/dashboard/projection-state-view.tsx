"use client";

import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { apiClient, type ProjectionKind, type ProjectionStateResponse } from "@/lib/api/client";

const BAR_FILL = "var(--color-primary)";

/** How many breakdown rows / chart buckets to show before collapsing the tail. */
const BREAKDOWN_TOP_N = 8;
const TIMESERIES_MAX_POINTS = 90;

function asNumber(v: unknown): number {
  return typeof v === "number" ? v : Number(v) || 0;
}

function asCountMap(v: unknown): Record<string, number> {
  if (!v || typeof v !== "object") return {};
  const out: Record<string, number> = {};
  for (const [k, val] of Object.entries(v as Record<string, unknown>)) {
    out[k] = asNumber(val);
  }
  return out;
}

function isEmptyState(kind: ProjectionKind | null, state: Record<string, unknown>): boolean {
  switch (kind) {
    case "counter":
      return asNumber(state.total) === 0;
    case "breakdown":
      return Object.keys(asCountMap(state.by_event_type)).length === 0;
    case "timeseries":
      return Object.keys(asCountMap(state.by_day)).length === 0;
    case "entity_table":
      // active-entities summary OR a single per-entity row
      if ("distinct" in state || "recent" in state) {
        return asNumber(state.distinct) === 0;
      }
      return asNumber(state.event_count) === 0;
    default:
      return Object.keys(state).length === 0;
  }
}

function CounterView({ state }: { state: Record<string, unknown> }) {
  const total = asNumber(state.total);
  const byType = asCountMap(state.by_event_type);
  const rows = Object.entries(byType).sort((a, b) => b[1] - a[1]);

  return (
    <div className="space-y-4">
      <div>
        <p className="text-3xl font-bold tabular-nums">{total.toLocaleString()}</p>
        <p className="text-xs text-muted-foreground">total events</p>
      </div>
      {rows.length > 0 && <BreakdownBars rows={rows} max={BREAKDOWN_TOP_N} valueLabel="events" />}
    </div>
  );
}

function BreakdownView({ state }: { state: Record<string, unknown> }) {
  const byType = asCountMap(state.by_event_type);
  const rows = Object.entries(byType).sort((a, b) => b[1] - a[1]);
  return <BreakdownBars rows={rows} max={BREAKDOWN_TOP_N} valueLabel="events" />;
}

function BreakdownBars({
  rows,
  max,
  valueLabel,
}: {
  rows: [string, number][];
  max: number;
  valueLabel: string;
}) {
  const top = rows.slice(0, max);
  const moreCount = rows.length - top.length;
  const peak = top[0]?.[1] ?? 0;

  return (
    <div className="space-y-2">
      {top.map(([label, count]) => (
        <div key={label} className="space-y-1">
          <div className="flex items-center justify-between gap-2 text-xs">
            <span className="truncate font-mono text-muted-foreground" title={label}>
              {label}
            </span>
            <span className="tabular-nums font-medium">{count.toLocaleString()}</span>
          </div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary"
              style={{ width: `${peak > 0 ? (count / peak) * 100 : 0}%` }}
            />
          </div>
        </div>
      ))}
      {moreCount > 0 && (
        <p className="pt-1 text-xs text-muted-foreground">
          +{moreCount} more {valueLabel === "events" ? "types" : "rows"}
        </p>
      )}
    </div>
  );
}

function TimeseriesView({ state }: { state: Record<string, unknown> }) {
  const byDay = asCountMap(state.by_day);
  const data = Object.entries(byDay)
    .sort((a, b) => a[0].localeCompare(b[0]))
    .slice(-TIMESERIES_MAX_POINTS)
    .map(([day, count]) => ({ day, count }));

  const total = data.reduce((sum, d) => sum + d.count, 0);

  return (
    <div className="space-y-2">
      <p className="text-xs text-muted-foreground">
        {total.toLocaleString()} events across {data.length} day{data.length === 1 ? "" : "s"}
      </p>
      <div className="h-32">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data} margin={{ top: 4, right: 4, left: 4, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" className="stroke-border" vertical={false} />
            <XAxis
              dataKey="day"
              fontSize={10}
              tick={{ fill: "var(--color-muted-foreground)" }}
              tickFormatter={(v: string) => v.slice(5)}
              interval="preserveStartEnd"
              minTickGap={24}
            />
            <YAxis
              fontSize={10}
              width={28}
              allowDecimals={false}
              tick={{ fill: "var(--color-muted-foreground)" }}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: "var(--color-card)",
                border: "1px solid var(--color-border)",
                borderRadius: "8px",
                color: "var(--color-card-foreground)",
                fontSize: "12px",
              }}
            />
            <Bar dataKey="count" fill={BAR_FILL} radius={[2, 2, 0, 0]} name="Events" />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function formatTs(ts: unknown): string {
  if (typeof ts !== "string" || !ts) return "—";
  const d = new Date(ts);
  return Number.isNaN(d.getTime()) ? ts : d.toLocaleString();
}

function EntityTableView({ state }: { state: Record<string, unknown> }) {
  // active-entities summary: { distinct, recent: { entity_id: last_event_at } }
  if ("distinct" in state || "recent" in state) {
    const recent = (
      state.recent && typeof state.recent === "object"
        ? (state.recent as Record<string, unknown>)
        : {}
    ) as Record<string, unknown>;
    const rows = Object.entries(recent)
      .map(([id, ts]) => [id, typeof ts === "string" ? ts : ""] as [string, string])
      .sort((a, b) => b[1].localeCompare(a[1]));

    return (
      <div className="space-y-3">
        <div>
          <p className="text-3xl font-bold tabular-nums">
            {asNumber(state.distinct).toLocaleString()}
          </p>
          <p className="text-xs text-muted-foreground">distinct entities</p>
        </div>
        {rows.length > 0 && (
          <div className="max-h-48 overflow-y-auto rounded-md border border-border">
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-muted/50 text-muted-foreground">
                <tr>
                  <th className="px-2 py-1.5 text-left font-medium">Entity</th>
                  <th className="px-2 py-1.5 text-right font-medium">Last active</th>
                </tr>
              </thead>
              <tbody>
                {rows.map(([id, ts]) => (
                  <tr key={id} className="border-t border-border">
                    <td className="truncate px-2 py-1.5 font-mono" title={id}>
                      {id}
                    </td>
                    <td className="whitespace-nowrap px-2 py-1.5 text-right text-muted-foreground">
                      {formatTs(ts)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    );
  }

  // Single per-entity row (entity-activity): { event_count, last_event_at, last_event_type }
  return (
    <dl className="grid grid-cols-2 gap-3 text-sm">
      <div>
        <dt className="text-xs text-muted-foreground">Events</dt>
        <dd className="font-medium tabular-nums">{asNumber(state.event_count).toLocaleString()}</dd>
      </div>
      <div>
        <dt className="text-xs text-muted-foreground">Last event</dt>
        <dd className="font-mono text-xs">{(state.last_event_type as string) || "—"}</dd>
      </div>
      <div className="col-span-2">
        <dt className="text-xs text-muted-foreground">Last seen</dt>
        <dd className="text-xs">{formatTs(state.last_event_at)}</dd>
      </div>
    </dl>
  );
}

function renderByKind(kind: ProjectionKind | null, state: Record<string, unknown>) {
  switch (kind) {
    case "counter":
      return <CounterView state={state} />;
    case "breakdown":
      return <BreakdownView state={state} />;
    case "timeseries":
      return <TimeseriesView state={state} />;
    case "entity_table":
      return <EntityTableView state={state} />;
    default:
      return (
        <pre className="overflow-x-auto rounded-md bg-muted p-2 text-xs">
          {JSON.stringify(state, null, 2)}
        </pre>
      );
  }
}

/**
 * Lazily fetches and renders a projection's folded read-model, formatted by its
 * `kind`. Mounted only when a Ready card is expanded, so state is never fetched
 * up front. Per-entity templates (entity_table without a tenant-wide summary)
 * accept an `entityId` filter.
 */
export function ProjectionStateView({
  name,
  kind,
  isPerEntity,
}: {
  name: string;
  kind: ProjectionKind | null;
  isPerEntity: boolean;
}) {
  const [entityId, setEntityId] = useState("");
  const [query, setQuery] = useState("");
  const [data, setData] = useState<ProjectionStateResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setNotFound(false);
    apiClient
      .getProjectionState(name, query || undefined)
      .then((res) => {
        if (cancelled) return;
        if (res.error) {
          setNotFound(true);
          setData(null);
        } else {
          setData(res.data ?? null);
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [name, query]);

  return (
    <div className="space-y-3 border-t border-border pt-4">
      {isPerEntity && (
        <form
          className="flex items-center gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            setQuery(entityId.trim());
          }}
        >
          <input
            type="text"
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            placeholder="Filter by entity_id…"
            className="h-8 flex-1 rounded-md border border-border bg-background px-2 text-xs"
          />
          <button
            type="submit"
            className="h-8 rounded-md border border-border px-3 text-xs hover:bg-muted"
          >
            View
          </button>
        </form>
      )}

      {loading ? (
        <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading state…
        </div>
      ) : notFound || !data ? (
        <p className="py-4 text-sm text-muted-foreground">
          {isPerEntity && query ? `No data folded for "${query}" yet` : "No data folded yet"}
        </p>
      ) : isEmptyState(data.kind ?? kind, data.state) ? (
        <p className="py-4 text-sm text-muted-foreground">No data folded yet</p>
      ) : (
        renderByKind(data.kind ?? kind, data.state)
      )}
    </div>
  );
}
