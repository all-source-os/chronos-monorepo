"use client";

import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Button } from "@allsource/ui";
import {
  TrendingUp,
  TrendingDown,
  Minus,
  Zap,
  HardDrive,
  Activity,
  RefreshCw,
  Pause,
  Play,
} from "lucide-react";
import { eventStoreClient } from "@/lib/event-store/client";
import type { Metrics } from "@/lib/event-store/types";

interface MetricHistory {
  timestamp: number;
  metrics: Metrics;
}

export function MetricsDemo() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const [refreshInterval, setRefreshInterval] = useState(5);
  const [metricHistory, setMetricHistory] = useState<MetricHistory[]>([]);

  const fetchMetrics = async () => {
    setIsLoading(true);
    try {
      const data = await eventStoreClient.getMetrics();
      setMetrics(data);

      // Add to history
      setMetricHistory((prev) => [
        { timestamp: Date.now(), metrics: data },
        ...prev.slice(0, 9), // Keep last 10 snapshots
      ]);
    } catch (error) {
      console.error("Failed to fetch metrics:", error);
    } finally {
      setIsLoading(false);
    }
  };

  // Initial fetch
  useEffect(() => {
    fetchMetrics();
  }, []);

  // Auto-refresh
  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      fetchMetrics();
    }, refreshInterval * 1000);

    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval]);

  // Calculate trend
  const getTrend = (currentValue: number, metricKey: keyof Metrics) => {
    if (metricHistory.length < 2) return { direction: "stable", change: 0 };

    const previousValue = metricHistory[1]?.metrics[metricKey];
    if (typeof previousValue !== "number") return { direction: "stable", change: 0 };

    const change = ((currentValue - previousValue) / previousValue) * 100;

    if (Math.abs(change) < 1) return { direction: "stable", change: 0 };
    return {
      direction: change > 0 ? "up" : "down",
      change: Math.abs(change),
    };
  };

  const metricCards = metrics
    ? [
        {
          label: "Ingestion Rate",
          value: metrics.ingestion_rate.toLocaleString(),
          unit: "/s",
          icon: Zap,
          color: "text-yellow-500",
          bgColor: "bg-yellow-500/10",
          borderColor: "border-yellow-500/20",
          trend: getTrend(metrics.ingestion_rate, "ingestion_rate"),
        },
        {
          label: "Query Latency",
          value: metrics.query_latency_ms.toFixed(2),
          unit: "ms",
          icon: Activity,
          color: "text-green-500",
          bgColor: "bg-green-500/10",
          borderColor: "border-green-500/20",
          trend: getTrend(metrics.query_latency_ms, "query_latency_ms"),
        },
        {
          label: "Active Connections",
          value: metrics.active_connections.toString(),
          unit: "",
          icon: TrendingUp,
          color: "text-blue-500",
          bgColor: "bg-blue-500/10",
          borderColor: "border-blue-500/20",
          trend: getTrend(metrics.active_connections, "active_connections"),
        },
        {
          label: "Storage Used",
          value: metrics.storage_used_gb.toFixed(2),
          unit: " GB",
          icon: HardDrive,
          color: "text-purple-500",
          bgColor: "bg-purple-500/10",
          borderColor: "border-purple-500/20",
          trend: getTrend(metrics.storage_used_gb, "storage_used_gb"),
        },
        {
          label: "Total Events",
          value: metrics.events_total.toLocaleString(),
          unit: "",
          icon: Activity,
          color: "text-pink-500",
          bgColor: "bg-pink-500/10",
          borderColor: "border-pink-500/20",
          trend: getTrend(metrics.events_total, "events_total"),
        },
      ]
    : [];

  const TrendIcon = ({ direction }: { direction: string }) => {
    if (direction === "up")
      return <TrendingUp className="h-3 w-3 text-green-400" />;
    if (direction === "down")
      return <TrendingDown className="h-3 w-3 text-red-400" />;
    return <Minus className="h-3 w-3 text-slate-500" />;
  };

  return (
    <div className="space-y-6">
      {/* Controls */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">System Metrics</h2>
        <div className="flex items-center gap-3">
          {/* Refresh Interval Selector */}
          <select
            value={refreshInterval}
            onChange={(e) => setRefreshInterval(Number(e.target.value))}
            disabled={isLoading}
            className="px-3 py-1 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={5}>5s</option>
            <option value={10}>10s</option>
            <option value={30}>30s</option>
            <option value={60}>60s</option>
          </select>

          {/* Auto-refresh Toggle */}
          <Button
            onClick={() => setAutoRefresh(!autoRefresh)}
            disabled={isLoading}
            variant="outline"
            size="sm"
            className={`${
              autoRefresh
                ? "bg-green-600/20 border-green-500 text-green-400"
                : "bg-slate-700 border-slate-600 text-white"
            }`}
          >
            {autoRefresh ? (
              <Pause className="h-4 w-4 mr-2" />
            ) : (
              <Play className="h-4 w-4 mr-2" />
            )}
            {autoRefresh ? "Auto" : "Manual"}
          </Button>

          {/* Manual Refresh */}
          <Button
            onClick={fetchMetrics}
            disabled={isLoading}
            className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
            size="sm"
          >
            <motion.div
              animate={isLoading ? { rotate: 360 } : {}}
              transition={{
                duration: 1,
                repeat: isLoading ? Number.POSITIVE_INFINITY : 0,
                ease: "linear",
              }}
            >
              <RefreshCw className="h-4 w-4" />
            </motion.div>
            <span className="ml-2">{isLoading ? "Refreshing..." : "Refresh"}</span>
          </Button>
        </div>
      </div>

      {/* Metric Cards */}
      {metrics ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {metricCards.map((metric, index) => (
            <motion.div
              key={metric.label}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.1 }}
              whileHover={{ scale: 1.02, y: -2 }}
              className={`p-6 rounded-xl border ${metric.borderColor} ${metric.bgColor} backdrop-blur-sm`}
            >
              <div className="flex items-start justify-between mb-4">
                <div className={`p-3 rounded-lg ${metric.bgColor}`}>
                  <metric.icon className={`h-6 w-6 ${metric.color}`} />
                </div>
                <div className="text-right">
                  <div className="flex items-center gap-2 justify-end mb-1">
                    <TrendIcon direction={metric.trend.direction} />
                    {metric.trend.change > 0 && (
                      <span className="text-xs text-slate-400">
                        {metric.trend.change.toFixed(1)}%
                      </span>
                    )}
                  </div>
                  <motion.div
                    key={metric.value}
                    initial={{ scale: 1.2 }}
                    animate={{ scale: 1 }}
                    className="flex items-baseline"
                  >
                    <p className="text-3xl font-bold text-white">{metric.value}</p>
                    {metric.unit && (
                      <span className="text-sm text-slate-400 ml-1">{metric.unit}</span>
                    )}
                  </motion.div>
                </div>
              </div>
              <p className="text-sm text-slate-300 font-medium">{metric.label}</p>

              {/* Mini Trend Chart */}
              {metricHistory.length > 1 && (
                <div className="mt-3 h-8 flex items-end gap-1">
                  {metricHistory
                    .slice(0, 10)
                    .reverse()
                    .map((history, idx) => {
                      const value = history.metrics[
                        metric.label === "Ingestion Rate"
                          ? "ingestion_rate"
                          : metric.label === "Query Latency"
                            ? "query_latency_ms"
                            : metric.label === "Active Connections"
                              ? "active_connections"
                              : metric.label === "Storage Used"
                                ? "storage_used_gb"
                                : "events_total"
                      ] as number;
                      const maxValue = Math.max(
                        ...metricHistory.map((h) => {
                          const metricKey =
                            metric.label === "Ingestion Rate"
                              ? "ingestion_rate"
                              : metric.label === "Query Latency"
                                ? "query_latency_ms"
                                : metric.label === "Active Connections"
                                  ? "active_connections"
                                  : metric.label === "Storage Used"
                                    ? "storage_used_gb"
                                    : "events_total";
                          return h.metrics[metricKey] as number;
                        })
                      );
                      const height = maxValue > 0 ? (value / maxValue) * 100 : 0;
                      return (
                        <motion.div
                          key={idx}
                          initial={{ height: 0 }}
                          animate={{ height: `${height}%` }}
                          transition={{ delay: idx * 0.05 }}
                          className={`flex-1 ${metric.bgColor} ${metric.borderColor} border-t-2 rounded-sm min-h-[4px]`}
                        />
                      );
                    })}
                </div>
              )}
            </motion.div>
          ))}
        </div>
      ) : (
        <div className="flex items-center justify-center py-12">
          <motion.div
            animate={{ opacity: [0.5, 1, 0.5] }}
            transition={{ duration: 1.5, repeat: Number.POSITIVE_INFINITY }}
            className="text-center"
          >
            <Activity className="h-12 w-12 mx-auto text-slate-600 mb-3" />
            <p className="text-slate-400">Loading metrics...</p>
          </motion.div>
        </div>
      )}

      {/* Status Bar */}
      {metrics && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.5 }}
          className="flex items-center justify-between p-4 bg-slate-800/50 rounded-lg border border-slate-700"
        >
          <div className="text-xs text-slate-500">
            Last updated: {new Date(metrics.timestamp).toLocaleString()}
          </div>
          {autoRefresh && (
            <div className="flex items-center gap-2 text-xs text-green-400">
              <div className="h-2 w-2 rounded-full bg-green-400 animate-pulse" />
              Auto-refreshing every {refreshInterval}s
            </div>
          )}
        </motion.div>
      )}

      {/* Metric History Summary */}
      {metrics && metricHistory.length > 1 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="p-4 bg-slate-800/50 rounded-lg border border-slate-700"
        >
          <h4 className="text-sm font-semibold text-white mb-3">Historical Snapshots</h4>
          <div className="grid grid-cols-2 md:grid-cols-5 gap-2 text-xs">
            <div className="p-2 bg-slate-900/30 rounded text-center">
              <p className="text-slate-400">Snapshots</p>
              <p className="text-white font-bold">{metricHistory.length}</p>
            </div>
            <div className="p-2 bg-slate-900/30 rounded text-center">
              <p className="text-slate-400">Avg Ingestion</p>
              <p className="text-white font-bold">
                {(
                  metricHistory.reduce((sum, h) => sum + h.metrics.ingestion_rate, 0) /
                  metricHistory.length
                ).toFixed(0)}
                /s
              </p>
            </div>
            <div className="p-2 bg-slate-900/30 rounded text-center">
              <p className="text-slate-400">Avg Latency</p>
              <p className="text-white font-bold">
                {(
                  metricHistory.reduce((sum, h) => sum + h.metrics.query_latency_ms, 0) /
                  metricHistory.length
                ).toFixed(1)}
                ms
              </p>
            </div>
            <div className="p-2 bg-slate-900/30 rounded text-center">
              <p className="text-slate-400">Peak Connections</p>
              <p className="text-white font-bold">
                {Math.max(...metricHistory.map((h) => h.metrics.active_connections))}
              </p>
            </div>
            <div className="p-2 bg-slate-900/30 rounded text-center">
              <p className="text-slate-400">Total Events</p>
              <p className="text-white font-bold">
                {metrics.events_total.toLocaleString()}
              </p>
            </div>
          </div>
        </motion.div>
      )}
    </div>
  );
}
