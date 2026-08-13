"use client";

import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle } from "@allsource/ui";
import { useCallback, useEffect, useState } from "react";
import { FadeIn } from "@/components/ui/fade-in";

// 15s matches the HTML status page served directly by Control Plane.
// CP probes every 10s internally, so 15s polling here stays fresh without
// ever trailing by more than one probe cycle.
const POLL_INTERVAL = 15_000;

// The CP feed returns these three status values. "stale" means a heartbeat
// hasn't arrived within heartbeatTTL (25s on CP); we render it as degraded
// since the probe is effectively missing, not confirmed down.
type HeartbeatStatus = "healthy" | "unhealthy" | "stale";

interface ServiceHeartbeat {
  service: string;
  status: HeartbeatStatus;
  latency_ms: number;
  last_seen: string;
  age_seconds: number;
  error?: string;
  probed_url?: string;
}

interface Incident {
  service: string;
  started_at: string;
  resolved_at: string | null;
  // Limited IP info: a redacted network prefix only (e.g. 203.0.113.0/24).
  observed_ip_prefix?: string | null;
}

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
}

function formatDuration(startIso: string, endIso: string | null): string {
  const start = new Date(startIso).getTime();
  const end = endIso ? new Date(endIso).getTime() : Date.now();
  const mins = Math.max(0, Math.round((end - start) / 60000));
  if (mins < 60) return `${mins}m`;
  return `${Math.floor(mins / 60)}h ${mins % 60}m`;
}

// Friendly labels + descriptions for the canonical service set. Services
// returned by CP that aren't in this map are still rendered, with the raw
// name as the label.
const SERVICE_METADATA: Record<string, { label: string; description: string }> = {
  "control-plane": {
    label: "Control Plane",
    description: "Public entry point — auth, delegation, quota, x402",
  },
  core: {
    label: "Core (Event Store)",
    description: "Rust event store — WAL, Parquet, 12μs reads",
  },
  query: {
    label: "Query Service",
    description: "Elixir API gateway — events, billing, WebSocket",
  },
  prime: {
    label: "Prime (Graph/Vector)",
    description: "Graph and vector store — semantic queries, recall",
  },
  auth: {
    label: "Auth Service",
    description: "Better-auth adapter over AllSource",
  },
  login: {
    label: "Login (end-to-end)",
    description: "Session-token validation — the exact path the dashboard login uses",
  },
  web: {
    label: "Website",
    description: "Marketing site and dashboard (Vercel)",
  },
};

function statusBadge(status: HeartbeatStatus | "checking") {
  switch (status) {
    case "healthy":
      return (
        <Badge className="bg-green-500/10 text-green-500 border-green-500/20">Operational</Badge>
      );
    case "unhealthy":
      return <Badge className="bg-red-500/10 text-red-500 border-red-500/20">Down</Badge>;
    case "stale":
      return (
        <Badge className="bg-yellow-500/10 text-yellow-500 border-yellow-500/20">Degraded</Badge>
      );
    case "checking":
      return <Badge className="bg-muted text-muted-foreground">Checking...</Badge>;
  }
}

function overallStatus(services: ServiceHeartbeat[]): { label: string; color: string } {
  if (services.length === 0) {
    return { label: "Checking systems...", color: "text-muted-foreground" };
  }
  const hasDown = services.some((s) => s.status === "unhealthy");
  const hasStale = services.some((s) => s.status === "stale");
  if (hasDown) return { label: "Partial Outage", color: "text-red-500" };
  if (hasStale) return { label: "Degraded Performance", color: "text-yellow-500" };
  return { label: "All Systems Operational", color: "text-green-500" };
}

function formatAge(ageSeconds: number): string {
  if (ageSeconds < 1) return "just now";
  if (ageSeconds < 60) return `${Math.round(ageSeconds)}s ago`;
  if (ageSeconds < 3600) return `${Math.round(ageSeconds / 60)}m ago`;
  return `${Math.round(ageSeconds / 3600)}h ago`;
}

export default function StatusPage() {
  const [services, setServices] = useState<ServiceHeartbeat[]>([]);
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [lastFetch, setLastFetch] = useState<Date | null>(null);
  const [fetchError, setFetchError] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const [svcRes, incRes] = await Promise.all([
        fetch("/api/status/services", { cache: "no-store" }),
        fetch("/api/status/incidents", { cache: "no-store" }),
      ]);
      const data = (await svcRes.json()) as { services?: ServiceHeartbeat[]; error?: string };
      setServices(data.services ?? []);
      setFetchError(data.error ?? null);
      const incData = (await incRes.json()) as { incidents?: Incident[] };
      setIncidents(incData.incidents ?? []);
      setLastFetch(new Date());
    } catch (err) {
      setFetchError(err instanceof Error ? err.message : String(err));
      setLastFetch(new Date());
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  const overall = overallStatus(services);

  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <FadeIn delay={0.1} inView>
        <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">System Status</h1>
        <p className={`text-lg font-medium mb-2 ${overall.color}`}>{overall.label}</p>
        <p className="text-sm text-muted-foreground mb-8">
          Powered by event-sourced heartbeats — every probe is a permanent event in Core.
        </p>
      </FadeIn>

      <FadeIn delay={0.2} inView>
        <Card>
          <CardHeader>
            <CardTitle>Services</CardTitle>
            <CardDescription>
              Probed every 10 seconds by Control Plane; refreshed here every 15 seconds
            </CardDescription>
          </CardHeader>
          <CardContent>
            {services.length === 0 && !fetchError ? (
              <p className="text-sm text-muted-foreground">Checking systems…</p>
            ) : services.length === 0 && fetchError ? (
              <p className="text-sm text-red-500">Unable to reach status feed: {fetchError}</p>
            ) : (
              <div className="space-y-4">
                {services.map((svc) => {
                  const meta = SERVICE_METADATA[svc.service];
                  return (
                    <div
                      key={svc.service}
                      className="flex items-center justify-between rounded-lg border p-4"
                    >
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-foreground">
                          {meta?.label ?? svc.service}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {meta?.description ?? svc.probed_url ?? svc.service}
                        </p>
                        {svc.status !== "healthy" && svc.error && (
                          <p className="text-xs text-red-500">{svc.error}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-4">
                        <span className="text-xs text-muted-foreground">{svc.latency_ms}ms</span>
                        <span className="text-xs text-muted-foreground w-20 text-right">
                          {formatAge(svc.age_seconds)}
                        </span>
                        {statusBadge(svc.status)}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      <FadeIn delay={0.3} inView>
        <Card className="mt-6">
          <CardHeader>
            <CardTitle>Incident History</CardTitle>
            <CardDescription>Last 30 days</CardDescription>
          </CardHeader>
          <CardContent>
            {incidents.length === 0 ? (
              <p className="text-sm text-muted-foreground">No incidents in the last 30 days.</p>
            ) : (
              <div className="space-y-4">
                {incidents.map((inc) => {
                  const meta = SERVICE_METADATA[inc.service];
                  const ongoing = !inc.resolved_at;
                  return (
                    <div
                      key={`${inc.service}-${inc.started_at}`}
                      className="flex items-start justify-between rounded-lg border p-4"
                    >
                      <div className="space-y-1">
                        <p className="text-sm font-medium text-foreground">
                          {meta?.label ?? inc.service} {ongoing ? "outage (ongoing)" : "outage"}
                        </p>
                        <p className="text-xs text-muted-foreground">
                          {formatTimestamp(inc.started_at)} ·{" "}
                          {formatDuration(inc.started_at, inc.resolved_at)}
                          {inc.observed_ip_prefix
                            ? ` · observed from ${inc.observed_ip_prefix}`
                            : ""}
                        </p>
                      </div>
                      {ongoing ? (
                        <Badge className="bg-red-500/10 text-red-500 border-red-500/20">
                          Ongoing
                        </Badge>
                      ) : (
                        <Badge className="bg-green-500/10 text-green-500 border-green-500/20">
                          Resolved
                        </Badge>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      {lastFetch && (
        <p className="text-xs text-muted-foreground mt-4 text-center">
          Last refreshed: {lastFetch.toLocaleTimeString()}
        </p>
      )}
    </div>
  );
}
