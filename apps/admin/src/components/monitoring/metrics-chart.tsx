"use client";

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { useState } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import type { TimeRange, TimeseriesPoint } from "@/lib/metrics-api";

interface MetricsChartProps {
  title: string;
  data: TimeseriesPoint[];
  dataKey?: string;
  color?: string;
  unit?: string;
  range: TimeRange;
  onRangeChange: (range: TimeRange) => void;
  isLoading?: boolean;
}

const ranges: { value: TimeRange; label: string }[] = [
  { value: "1h", label: "1H" },
  { value: "24h", label: "24H" },
  { value: "7d", label: "7D" },
];

export function MetricsChart({
  title,
  data,
  color = "hsl(var(--primary))",
  unit = "",
  range,
  onRangeChange,
  isLoading,
}: MetricsChartProps) {
  const chartConfig: ChartConfig = {
    value: {
      label: title,
      color,
    },
  };

  const chartData = data.map((point) => ({
    time: formatTime(point.timestamp, range),
    value: point.value,
  }));

  return (
    <Card data-testid={`chart-${title.toLowerCase().replace(/[\s/]+/g, "-")}`}>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-base font-medium">{title}</CardTitle>
        <div className="flex gap-1">
          {ranges.map((r) => (
            <button
              type="button"
              key={r.value}
              onClick={() => onRangeChange(r.value)}
              className={cn(
                "rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                range === r.value
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground"
              )}
            >
              {r.label}
            </button>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex h-[250px] items-center justify-center">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          </div>
        ) : chartData.length === 0 ? (
          <div className="flex h-[250px] items-center justify-center text-sm text-muted-foreground">
            No data available
          </div>
        ) : (
          <ChartContainer config={chartConfig} className="h-[250px] w-full">
            <AreaChart data={chartData}>
              <defs>
                <linearGradient id={`fill-${title}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor={color} stopOpacity={0.3} />
                  <stop offset="95%" stopColor={color} stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" vertical={false} />
              <XAxis
                dataKey="time"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                fontSize={12}
              />
              <YAxis
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                fontSize={12}
                tickFormatter={(v: number) => `${v}${unit}`}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    formatter={(value) => (
                      <span className="font-mono font-medium">
                        {typeof value === "number" ? value.toFixed(2) : value}
                        {unit}
                      </span>
                    )}
                  />
                }
              />
              <Area
                type="monotone"
                dataKey="value"
                stroke={color}
                fill={`url(#fill-${title})`}
                strokeWidth={2}
              />
            </AreaChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  );
}

function formatTime(timestamp: string, range: TimeRange): string {
  const date = new Date(timestamp);
  if (range === "1h") {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  if (range === "24h") {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}
