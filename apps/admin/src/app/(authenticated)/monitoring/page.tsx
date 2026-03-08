"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  Activity,
  Bell,
  Clock,
  Gauge,
  AlertTriangle,
  Target,
  Users,
  Zap,
  RefreshCw,
} from "lucide-react";
import Link from "next/link";
import { cn } from "@allsource/ui/utils";
import { StatCard } from "@/components/monitoring/stat-card";
import { MetricsChart } from "@/components/monitoring/metrics-chart";
import { ClusterHealth } from "@/components/monitoring/cluster-health";
import {
  fetchMetricsSummary,
  fetchTimeseries,
  fetchClusterMembers,
  formatUptime,
  type MetricsSummary,
  type TimeseriesPoint,
  type ClusterMember,
  type TimeRange,
} from "@/lib/metrics-api";

const AUTO_REFRESH_INTERVAL = 30_000;

export default function MonitoringPage() {
  const [summary, setSummary] = useState<MetricsSummary | null>(null);
  const [throughputData, setThroughputData] = useState<TimeseriesPoint[]>([]);
  const [errorRateData, setErrorRateData] = useState<TimeseriesPoint[]>([]);
  const [clusterMembers, setClusterMembers] = useState<ClusterMember[]>([]);

  const [throughputRange, setThroughputRange] = useState<TimeRange>("1h");
  const [errorRateRange, setErrorRateRange] = useState<TimeRange>("1h");

  const [isLoading, setIsLoading] = useState(true);
  const [isChartsLoading, setIsChartsLoading] = useState(false);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [secondsUntilRefresh, setSecondsUntilRefresh] = useState(
    AUTO_REFRESH_INTERVAL / 1000
  );

  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const countdownRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadSummaryAndCluster = useCallback(async () => {
    try {
      const [summaryData, members] = await Promise.all([
        fetchMetricsSummary(),
        fetchClusterMembers(),
      ]);
      setSummary(summaryData);
      setClusterMembers(members);
    } catch (err) {
      console.error("Failed to load metrics summary:", err);
    }
  }, []);

  const loadTimeseries = useCallback(
    async (tRange: TimeRange, eRange: TimeRange) => {
      setIsChartsLoading(true);
      try {
        const [throughput, errorRate] = await Promise.all([
          fetchTimeseries("events_per_second", tRange),
          fetchTimeseries("error_rate_percent", eRange),
        ]);
        setThroughputData(throughput);
        setErrorRateData(errorRate);
      } catch (err) {
        console.error("Failed to load timeseries:", err);
      } finally {
        setIsChartsLoading(false);
      }
    },
    []
  );

  const refreshAll = useCallback(async () => {
    await Promise.all([
      loadSummaryAndCluster(),
      loadTimeseries(throughputRange, errorRateRange),
    ]);
    setLastRefresh(new Date());
    setSecondsUntilRefresh(AUTO_REFRESH_INTERVAL / 1000);
  }, [loadSummaryAndCluster, loadTimeseries, throughputRange, errorRateRange]);

  // Initial load
  useEffect(() => {
    const init = async () => {
      setIsLoading(true);
      await refreshAll();
      setIsLoading(false);
    };
    init();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-refresh timer
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

  // Reload timeseries when range changes
  useEffect(() => {
    loadTimeseries(throughputRange, errorRateRange);
  }, [throughputRange, errorRateRange, loadTimeseries]);

  const handleManualRefresh = () => {
    setSecondsUntilRefresh(AUTO_REFRESH_INTERVAL / 1000);
    refreshAll();
  };

  const errorStatus =
    summary && summary.error_rate_percent > 5
      ? "critical"
      : summary && summary.error_rate_percent > 1
        ? "warning"
        : "healthy";

  const latencyStatus =
    summary && summary.query_latency_p99_ms > 500
      ? "critical"
      : summary && summary.query_latency_p99_ms > 200
        ? "warning"
        : "healthy";

  return (
    <div className="space-y-6" data-testid="monitoring-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Monitoring</h1>
          <p className="text-muted-foreground">
            Platform health, Core cluster status, and performance metrics.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <div
            className="flex items-center gap-2 text-xs text-muted-foreground"
            data-testid="auto-refresh-indicator"
          >
            <div
              className={cn(
                "h-2 w-2 rounded-full",
                isLoading || isChartsLoading
                  ? "animate-pulse bg-yellow-500"
                  : "bg-green-500"
              )}
            />
            <span>
              {isLoading || isChartsLoading
                ? "Refreshing..."
                : `Next refresh in ${secondsUntilRefresh}s`}
            </span>
          </div>
          <button
            type="button"
            onClick={handleManualRefresh}
            className="rounded-md p-2 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
            aria-label="Refresh metrics"
          >
            <RefreshCw
              className={cn(
                "h-4 w-4",
                (isLoading || isChartsLoading) && "animate-spin"
              )}
            />
          </button>
        </div>
      </div>

      {/* Sub-navigation */}
      <div className="flex gap-3" data-testid="monitoring-subnav">
        <Link
          href="/monitoring/alerts"
          className="flex items-center gap-2 rounded-lg border border-border bg-card px-4 py-3 text-sm font-medium transition-colors hover:bg-muted"
          data-testid="nav-alerts"
        >
          <Bell className="h-4 w-4 text-muted-foreground" />
          Alert Rules
        </Link>
        <Link
          href="/monitoring/slos"
          className="flex items-center gap-2 rounded-lg border border-border bg-card px-4 py-3 text-sm font-medium transition-colors hover:bg-muted"
          data-testid="nav-slos"
        >
          <Target className="h-4 w-4 text-muted-foreground" />
          SLOs
        </Link>
      </div>

      {/* Stat cards */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5" data-testid="stat-cards">
        <StatCard
          title="Uptime"
          value={summary ? formatUptime(summary.uptime_seconds) : "--"}
          icon={Clock}
          status="healthy"
        />
        <StatCard
          title="Events/sec"
          value={
            summary ? summary.events_per_second.toLocaleString(undefined, { maximumFractionDigits: 1 }) : "--"
          }
          subtitle={
            summary
              ? `${summary.events_total.toLocaleString()} total events`
              : undefined
          }
          icon={Zap}
          status="healthy"
        />
        <StatCard
          title="Latency p99"
          value={summary ? `${summary.query_latency_p99_ms.toFixed(1)}ms` : "--"}
          icon={Gauge}
          status={latencyStatus}
        />
        <StatCard
          title="Error Rate"
          value={
            summary
              ? `${summary.error_rate_percent.toFixed(2)}%`
              : "--"
          }
          icon={AlertTriangle}
          status={errorStatus}
        />
        <StatCard
          title="Active Tenants"
          value={summary ? summary.active_tenants.toLocaleString() : "--"}
          icon={Users}
          status="healthy"
        />
      </div>

      {/* Charts */}
      <div className="grid gap-6 lg:grid-cols-2" data-testid="charts-section">
        <MetricsChart
          title="Throughput"
          data={throughputData}
          color="hsl(142, 76%, 36%)"
          unit=" evt/s"
          range={throughputRange}
          onRangeChange={setThroughputRange}
          isLoading={isChartsLoading}
        />
        <MetricsChart
          title="Error Rate"
          data={errorRateData}
          color="hsl(0, 84%, 60%)"
          unit="%"
          range={errorRateRange}
          onRangeChange={setErrorRateRange}
          isLoading={isChartsLoading}
        />
      </div>

      {/* Cluster health */}
      <ClusterHealth members={clusterMembers} isLoading={isLoading} />
    </div>
  );
}
