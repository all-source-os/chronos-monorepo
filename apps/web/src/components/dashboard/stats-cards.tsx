"use client";

import { Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Activity,
  Database,
  GitBranch,
  RefreshCw,
  TrendingDown,
  TrendingUp,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useDashboardStats } from "@/hooks/use-dashboard-stats";

interface StatCardProps {
  title: string;
  value: string | number;
  icon: React.ElementType;
  trend?: {
    value: number;
    isPositive: boolean;
  };
  description?: string;
  animate?: boolean;
}

function StatCard({ title, value, icon: Icon, trend, description, animate }: StatCardProps) {
  const [displayValue, setDisplayValue] = useState(animate ? 0 : value);

  useEffect(() => {
    if (!animate || typeof value !== "number") {
      setDisplayValue(value);
      return;
    }

    const duration = 1000;
    const steps = 30;
    const increment = value / steps;
    let current = 0;
    const interval = duration / steps;

    const timer = setInterval(() => {
      current += increment;
      if (current >= value) {
        setDisplayValue(value);
        clearInterval(timer);
      } else {
        setDisplayValue(Math.floor(current));
      }
    }, interval);

    return () => clearInterval(timer);
  }, [value, animate]);

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-start justify-between">
          <div className="space-y-2">
            <p className="text-sm font-medium text-muted-foreground">{title}</p>
            <div className="flex items-baseline gap-2">
              <span className="text-3xl font-bold tracking-tight">
                {typeof displayValue === "number" ? displayValue.toLocaleString() : displayValue}
              </span>
              {trend && (
                <span
                  className={cn(
                    "flex items-center text-xs font-medium",
                    trend.isPositive ? "text-green-600" : "text-red-600"
                  )}
                >
                  {trend.isPositive ? (
                    <TrendingUp className="mr-0.5 h-3 w-3" />
                  ) : (
                    <TrendingDown className="mr-0.5 h-3 w-3" />
                  )}
                  {trend.value}%
                </span>
              )}
            </div>
            {description && <p className="text-xs text-muted-foreground">{description}</p>}
          </div>
          <div className="rounded-lg bg-primary/10 p-3">
            <Icon className="h-5 w-5 text-primary" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export function StatsCards() {
  const { stats, isLoading, refresh } = useDashboardStats();

  const statCards = [
    {
      // REAL tenant-scoped total from the event store (summed per-type counts),
      // not the billing meter — which reads 0 for out-of-band ingestion.
      title: "Total Events",
      value: stats.events.total,
      icon: Activity,
      description: `${stats.store.streams.toLocaleString()} streams · ${stats.store.eventTypes.toLocaleString()} event types`,
      animate: true,
    },
    {
      title: "Active Projections",
      value: stats.projections.active,
      icon: GitBranch,
      description: `${stats.projections.count} total projections`,
      animate: true,
    },
    {
      title: "Event Streams",
      value: stats.store.streams,
      icon: Database,
      description: "Distinct entities in your store",
      animate: true,
    },
    {
      // Platform/system p99 across all query types, derived from Core's
      // query-duration histogram (global, all tenants) — labelled as a platform
      // metric, not "your" latency. Honest "—" when no queries observed yet.
      title: "p99 Latency",
      value: stats.latency.formatted,
      icon: Zap,
      description:
        stats.latency.formatted === "—" ? "Awaiting query samples" : "Platform query latency",
    },
  ];

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-end">
        <button
          onClick={refresh}
          disabled={isLoading}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground disabled:opacity-50"
        >
          <RefreshCw className={cn("h-3 w-3", isLoading && "animate-spin")} />
          {isLoading ? "Refreshing..." : "Refresh"}
        </button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {statCards.map((stat) => (
          <StatCard key={stat.title} {...stat} />
        ))}
      </div>
    </div>
  );
}
