"use client";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Activity,
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  ChevronRight,
  RefreshCw,
  ShieldAlert,
} from "lucide-react";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { HealthChip } from "@/components/fleet/health-chip";
import { StatCard } from "@/components/monitoring/stat-card";
import { type FleetHealthSummary, type FleetWorstTenant, fetchFleetHealth } from "@/lib/fleet-api";

const AUTO_REFRESH_INTERVAL = 30_000;

// ── Fixture (clearly marked) ──────────────────────────────────────────
// The Control Plane fleet endpoints (commit a02667e) are NOT yet deployed.
// This fixture lets the page render its real layout when the API is
// unreachable. fetchFleetHealth() still calls the documented route exactly;
// this only kicks in on fetch failure. Remove once the endpoints are live.
const FIXTURE_FLEET: FleetHealthSummary = {
  summary: { total: 12, healthy: 7, degraded: 2, at_risk: 2, critical: 1 },
  worst: [
    {
      tenant_id: "tenant-acme",
      name: "Acme Corp",
      tier: "critical",
      reasons: [
        { signal: "subscription_state", value: "canceled", tier: "critical" },
        { signal: "events_quota_pct", value: "118%", tier: "at_risk" },
      ],
    },
    {
      tenant_id: "tenant-globex",
      name: "Globex",
      tier: "at_risk",
      reasons: [
        { signal: "events_quota_pct", value: "104%", tier: "at_risk" },
        { signal: "last_sync_age", value: "31h", tier: "at_risk" },
      ],
    },
    {
      tenant_id: "tenant-initech",
      name: "Initech",
      tier: "at_risk",
      reasons: [{ signal: "last_event_age", value: "9d", tier: "at_risk" }],
    },
    {
      tenant_id: "tenant-umbrella",
      name: "Umbrella",
      tier: "degraded",
      reasons: [{ signal: "subscription_state", value: "past_due", tier: "degraded" }],
    },
    {
      tenant_id: "tenant-hooli",
      name: "Hooli",
      tier: "degraded",
      reasons: [{ signal: "events_quota_pct", value: "87%", tier: "degraded" }],
    },
  ],
};

export default function FleetPage() {
  const [data, setData] = useState<FleetHealthSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [usingFixture, setUsingFixture] = useState(false);
  const [secondsUntilRefresh, setSecondsUntilRefresh] = useState(AUTO_REFRESH_INTERVAL / 1000);

  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const countdownRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refreshAll = useCallback(async () => {
    try {
      const fleet = await fetchFleetHealth({ limit: 25 });
      setData(fleet);
      setUsingFixture(false);
    } catch (err) {
      // Endpoint not yet deployed — fall back to the marked fixture so the
      // page still renders its layout for verification.
      console.error("Failed to load fleet health (using fixture):", err);
      setData(FIXTURE_FLEET);
      setUsingFixture(true);
    } finally {
      setSecondsUntilRefresh(AUTO_REFRESH_INTERVAL / 1000);
    }
  }, []);

  // Initial load
  // biome-ignore lint/correctness/useExhaustiveDependencies: run once on mount; refreshAll is stable.
  useEffect(() => {
    const init = async () => {
      setIsLoading(true);
      await refreshAll();
      setIsLoading(false);
    };
    init();
  }, []);

  // Auto-refresh timer (monitoring/page.tsx pattern verbatim)
  useEffect(() => {
    refreshTimerRef.current = setInterval(() => {
      refreshAll();
    }, AUTO_REFRESH_INTERVAL);

    countdownRef.current = setInterval(() => {
      setSecondsUntilRefresh((prev) => Math.max(0, prev - 1));
    }, 1000);

    return () => {
      if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
      if (countdownRef.current) clearInterval(countdownRef.current);
    };
  }, [refreshAll]);

  const handleManualRefresh = () => {
    setSecondsUntilRefresh(AUTO_REFRESH_INTERVAL / 1000);
    refreshAll();
  };

  const summary = data?.summary;
  const worst = data?.worst ?? [];

  return (
    <div className="space-y-6" data-testid="fleet-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Fleet Health</h1>
          <p className="text-muted-foreground">
            Every tenant&apos;s health at a glance. Drill into any non-green tenant to see why and
            run guarded recovery.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <div
            className="flex items-center gap-2 text-xs text-muted-foreground"
            data-testid="fleet-auto-refresh-indicator"
          >
            <div
              className={cn(
                "h-2 w-2 rounded-full",
                isLoading ? "animate-pulse bg-yellow-500" : "bg-green-500"
              )}
            />
            <span>{isLoading ? "Refreshing…" : `Next refresh in ${secondsUntilRefresh}s`}</span>
          </div>
          <button
            type="button"
            onClick={handleManualRefresh}
            className="rounded-md p-2 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
            aria-label="Refresh fleet health"
            data-testid="fleet-refresh-btn"
          >
            <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
          </button>
        </div>
      </div>

      {usingFixture && (
        <div
          className="flex items-center gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-600"
          data-testid="fleet-fixture-banner"
        >
          <AlertTriangle className="h-4 w-4 shrink-0" />
          <span>
            <strong>FIXTURE — endpoint unreachable.</strong> This is sample data, NOT live fleet
            health. The Control Plane fleet endpoint did not answer; numbers below are placeholders.
          </span>
        </div>
      )}

      {/* Stat cards — four tier counts */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4" data-testid="fleet-stat-cards">
        <StatCard
          title="Healthy"
          value={summary ? summary.healthy.toLocaleString() : "--"}
          subtitle={summary ? `of ${summary.total} tenants` : undefined}
          icon={CheckCircle2}
          status="healthy"
        />
        <StatCard
          title="Degraded"
          value={summary ? summary.degraded.toLocaleString() : "--"}
          icon={AlertTriangle}
          status={summary && summary.degraded > 0 ? "warning" : "healthy"}
        />
        <StatCard
          title="At-Risk"
          value={summary ? summary.at_risk.toLocaleString() : "--"}
          icon={AlertCircle}
          status={summary && summary.at_risk > 0 ? "critical" : "healthy"}
        />
        <StatCard
          title="Critical"
          value={summary ? summary.critical.toLocaleString() : "--"}
          icon={ShieldAlert}
          status={summary && summary.critical > 0 ? "critical" : "healthy"}
        />
      </div>

      {/* Worst-N table */}
      <Card data-testid="fleet-worst-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="h-5 w-5" />
            Tenants Needing Attention
          </CardTitle>
          <CardDescription>
            Worst-ranked tenants (Critical → Degraded). Each row lists the signals that triggered
            its tier.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex h-32 items-center justify-center text-sm text-muted-foreground">
              Loading fleet health…
            </div>
          ) : worst.length === 0 ? (
            <div
              className="flex h-32 items-center justify-center text-sm text-muted-foreground"
              data-testid="fleet-all-healthy"
            >
              All tenants are healthy. Nothing to recover.
            </div>
          ) : (
            <Table data-testid="fleet-worst-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Tenant</TableHead>
                  <TableHead>Health</TableHead>
                  <TableHead>Contributing signals</TableHead>
                  <TableHead className="text-right" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {worst.map((t: FleetWorstTenant) => (
                  <TableRow
                    key={t.tenant_id}
                    className="cursor-pointer"
                    data-testid={`fleet-row-${t.tenant_id}`}
                  >
                    <TableCell>
                      <Link href={`/fleet/${t.tenant_id}`} className="block hover:underline">
                        <p className="font-medium">{t.name || t.tenant_id}</p>
                        <p className="text-xs text-muted-foreground font-mono">{t.tenant_id}</p>
                      </Link>
                    </TableCell>
                    <TableCell>
                      <HealthChip tier={t.tier} />
                    </TableCell>
                    <TableCell>
                      <ul className="space-y-1">
                        {t.reasons.map((r) => (
                          <li
                            key={`${t.tenant_id}-${r.signal}`}
                            className="flex items-center gap-2 text-xs"
                            data-testid={`fleet-reason-${t.tenant_id}-${r.signal}`}
                          >
                            <HealthChip tier={r.tier} dotOnly />
                            <span className="font-mono text-muted-foreground">{r.signal}</span>
                            <span className="font-medium">{r.value}</span>
                          </li>
                        ))}
                      </ul>
                    </TableCell>
                    <TableCell className="text-right">
                      <Link
                        href={`/fleet/${t.tenant_id}`}
                        className="inline-flex items-center text-sm text-muted-foreground hover:text-foreground"
                        data-testid={`fleet-drilldown-${t.tenant_id}`}
                      >
                        Drill in
                        <ChevronRight className="h-4 w-4" />
                      </Link>
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
