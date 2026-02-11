"use client";

import { useEffect, useState } from "react";
import { Activity, Database, Zap, GitBranch, TrendingUp, TrendingDown } from "lucide-react";
import { Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { useAuthStore } from "@/lib/stores/auth-store";

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
                {typeof displayValue === "number"
                  ? displayValue.toLocaleString()
                  : displayValue}
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
            {description && (
              <p className="text-xs text-muted-foreground">{description}</p>
            )}
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
  const { tenant } = useAuthStore();

  const stats = [
    {
      title: "Total Events",
      value: tenant?.events_used || 0,
      icon: Activity,
      trend: { value: 12, isPositive: true },
      description: `of ${(tenant?.events_quota || 0).toLocaleString()} quota`,
      animate: true,
    },
    {
      title: "Queries Executed",
      value: tenant?.queries_used || 0,
      icon: Database,
      trend: { value: 8, isPositive: true },
      description: `of ${(tenant?.queries_quota || 0).toLocaleString()} quota`,
      animate: true,
    },
    {
      title: "Active Projections",
      value: 3,
      icon: GitBranch,
      description: "Real-time data views",
      animate: true,
    },
    {
      title: "Avg Latency",
      value: "11.9μs",
      icon: Zap,
      trend: { value: 5, isPositive: true },
      description: "p99 query response",
    },
  ];

  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      {stats.map((stat) => (
        <StatCard key={stat.title} {...stat} />
      ))}
    </div>
  );
}
