"use client";

import {
  Badge,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { Activity, Database, Eye, HardDrive, Zap } from "lucide-react";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { ViewAsBanner } from "@/components/tenants/view-as-banner";
import { getViewAsSession, type ViewAsSession } from "@/lib/viewas-api";

/**
 * Read-only product frame for "view as tenant" (ADMIN_TENANT_POWER_TOOL §5.2/§5.3).
 *
 * This is the read-only surface (layer 3 of the three read-only enforcement
 * layers): it renders the TENANT's world — their event store stats + recent
 * events — exactly as the tenant would see them, with NO write affordances. All
 * reads go through the read-only data proxy (GET /api/viewas/data/*), which
 * attaches the scoped `viewas_token` and refuses any mutating method. The real
 * enforcement is server-side (the readonly role + the CP view_as write-refusal);
 * this surface simply exposes no mutation UI.
 *
 * The persistent banner (mounted at the top) carries the live countdown + Exit +
 * the auto-expiry teardown. When the session is gone (expired/torn down) the data
 * reads 401 and the page shows the inactive state; the banner's expiry path
 * returns the operator to the admin frame.
 *
 * Resilience (§6): the page sits under (authenticated)/error.tsx; every list runs
 * through a local array guard; every number is `?? 0`; data reads settle
 * independently and never crash the frame.
 */

interface TenantStats {
  tenant_id?: string;
  event_count?: number;
  stream_count?: number;
  storage_bytes?: number;
}

interface EventRow {
  id?: string;
  event_id?: string;
  event_type?: string;
  entity_id?: string;
  timestamp?: string;
  version?: number;
}

async function readViewAs<T>(path: string): Promise<T> {
  const res = await fetch(`/api/viewas/data/${path}`, {
    credentials: "include",
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`view-as read failed (${res.status}): ${path}`);
  }
  return res.json();
}

function asArray<T>(data: unknown, ...keys: string[]): T[] {
  if (Array.isArray(data)) return data as T[];
  if (data && typeof data === "object") {
    for (const k of [...keys, "items", "data"]) {
      const v = (data as Record<string, unknown>)[k];
      if (Array.isArray(v)) return v as T[];
    }
  }
  return [];
}

function formatDateTime(iso?: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export default function ViewAsFramePage() {
  const params = useParams();
  const tenantId = params.id as string;

  const [session, setSession] = useState<ViewAsSession | null>(null);
  const [sessionChecked, setSessionChecked] = useState(false);
  const [stats, setStats] = useState<TenantStats | null>(null);
  const [events, setEvents] = useState<EventRow[]>([]);
  const [loadingData, setLoadingData] = useState(true);
  const [dataError, setDataError] = useState<string | null>(null);

  // Resolve the active view-as session (drives the banner). If there's no session
  // — never started, expired, or torn down — render the inactive state.
  useEffect(() => {
    let cancelled = false;
    getViewAsSession()
      .then((s) => {
        if (!cancelled) setSession(s);
      })
      .finally(() => {
        if (!cancelled) setSessionChecked(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const loadData = useCallback(async () => {
    setLoadingData(true);
    setDataError(null);
    // The tenant-scoped reads through the view-as token. Each settles
    // independently so one unreachable endpoint can't blank the whole frame.
    const [statsRes, eventsRes] = await Promise.allSettled([
      readViewAs<TenantStats>(`tenants/${encodeURIComponent(tenantId)}/stats`),
      readViewAs<{ events?: EventRow[] }>(`events/query?limit=25&order=desc`),
    ]);

    if (statsRes.status === "fulfilled") {
      setStats(statsRes.value);
    } else {
      setStats(null);
      // If the stats read failed because the session is inactive (401), surface it.
      setDataError(
        statsRes.reason instanceof Error ? statsRes.reason.message : "Stats unavailable."
      );
    }

    setEvents(eventsRes.status === "fulfilled" ? asArray<EventRow>(eventsRes.value, "events") : []);
    setLoadingData(false);
  }, [tenantId]);

  // Only load tenant data once we've confirmed an active session.
  useEffect(() => {
    if (sessionChecked && session) {
      loadData();
    } else if (sessionChecked && !session) {
      setLoadingData(false);
    }
  }, [sessionChecked, session, loadData]);

  // No active session — the operator is not (or no longer) viewing as this tenant.
  if (sessionChecked && !session) {
    return (
      <div
        className="flex min-h-[50vh] flex-col items-center justify-center gap-3 text-center"
        data-testid="viewas-inactive"
      >
        <Eye className="h-8 w-8 text-muted-foreground" />
        <h2 className="text-lg font-semibold">No active view-as session</h2>
        <p className="max-w-md text-sm text-muted-foreground">
          The read-only view-as session has ended or expired. Return to the tenant to start a new
          one.
        </p>
        <a
          href={`/tenants/${tenantId}`}
          className="rounded-md border px-4 py-2 text-sm font-medium transition-colors hover:bg-muted"
          data-testid="viewas-back-link"
        >
          Back to tenant
        </a>
      </div>
    );
  }

  const eventCount = stats?.event_count ?? 0;
  const streamCount = stats?.stream_count ?? 0;
  const storageBytes = stats?.storage_bytes ?? 0;
  const storageMb = storageBytes / (1024 * 1024);

  return (
    <div className="space-y-6" data-testid="viewas-frame">
      {/* Persistent banner — countdown + Exit + auto-expiry teardown. Rendered as
          soon as the session is known so it's unmissable for the whole frame. */}
      {session && <ViewAsBanner session={session} returnTo={`/tenants/${tenantId}`} />}

      <div className="flex items-center gap-3">
        <Eye className="h-6 w-6 text-amber-600 dark:text-amber-400" />
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            {session?.tenant_name || "Tenant"} — read-only view
          </h1>
          <p className="text-sm text-muted-foreground">
            You are seeing this tenant&apos;s product exactly as they do. This frame is read-only —
            there is no write path.
          </p>
        </div>
      </div>

      {/* Stats — the tenant's event store at a glance (read-only). */}
      <div className="grid gap-4 sm:grid-cols-3" data-testid="viewas-stats">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Zap className="h-4 w-4" />
              Events
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums" data-testid="viewas-event-count">
              {eventCount.toLocaleString()}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Database className="h-4 w-4" />
              Streams
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums" data-testid="viewas-stream-count">
              {streamCount.toLocaleString()}
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <HardDrive className="h-4 w-4" />
              Storage
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums" data-testid="viewas-storage">
              {storageMb.toLocaleString(undefined, { maximumFractionDigits: 1 })} MB
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Recent events — what the tenant's stream looks like right now. */}
      <Card data-testid="viewas-events-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Recent events
          </CardTitle>
          <CardDescription>
            Newest events in this tenant&apos;s store (read-only — via the scoped view-as token).
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadingData ? (
            <div className="space-y-2" data-testid="viewas-events-loading">
              {["a", "b", "c", "d"].map((k) => (
                <Skeleton key={`vs-skel-${k}`} className="h-10 w-full" />
              ))}
            </div>
          ) : events.length === 0 ? (
            <p
              className="py-8 text-center text-sm text-muted-foreground"
              data-testid="viewas-events-empty"
            >
              {dataError
                ? "Could not load this tenant's events with the view-as session."
                : "No events in this tenant's store."}
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Type</TableHead>
                  <TableHead>Entity</TableHead>
                  <TableHead className="text-right">Version</TableHead>
                  <TableHead>Timestamp</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {events.map((ev, i) => (
                  <TableRow
                    key={ev.id || ev.event_id || `${ev.entity_id}-${ev.version}-${i}`}
                    data-testid="viewas-event-row"
                  >
                    <TableCell>
                      <Badge variant="outline">{ev.event_type || "—"}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-xs">{ev.entity_id || "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">{ev.version ?? "—"}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {formatDateTime(ev.timestamp)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
