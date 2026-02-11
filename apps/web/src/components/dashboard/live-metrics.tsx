"use client";

import { useEffect, useState, useRef } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Activity, Gauge, Zap, Pause, Play } from "lucide-react";
import { Button } from "@allsource/ui";

interface MetricData {
  eventsPerSec: number;
  latencyP99: number;
  throughput: number;
}

export function LiveMetrics() {
  const [isPaused, setIsPaused] = useState(false);
  const [metrics, setMetrics] = useState<MetricData>({
    eventsPerSec: 469000,
    latencyP99: 11.9,
    throughput: 98.7,
  });
  const [sparklineData, setSparklineData] = useState<number[]>([]);
  const animationRef = useRef<NodeJS.Timeout | null>(null);

  // Simulate real-time metrics (in production, this would use WebSocket)
  useEffect(() => {
    if (isPaused) return;

    animationRef.current = setInterval(() => {
      setMetrics((prev) => ({
        eventsPerSec: Math.floor(
          469000 + (Math.random() - 0.5) * 50000
        ),
        latencyP99: Math.round((11.9 + (Math.random() - 0.5) * 2) * 10) / 10,
        throughput: Math.round((98.7 + (Math.random() - 0.5) * 2) * 10) / 10,
      }));

      setSparklineData((prev) => {
        const newData = [...prev, 469000 + (Math.random() - 0.5) * 50000];
        return newData.slice(-30); // Keep last 30 data points
      });
    }, 1000);

    return () => {
      if (animationRef.current) clearInterval(animationRef.current);
    };
  }, [isPaused]);

  // Initialize sparkline data
  useEffect(() => {
    const initialData = Array.from({ length: 30 }, () =>
      Math.floor(469000 + (Math.random() - 0.5) * 50000)
    );
    setSparklineData(initialData);
  }, []);

  const maxSparkline = Math.max(...sparklineData, 1);
  const minSparkline = Math.min(...sparklineData, 0);
  const range = maxSparkline - minSparkline || 1;

  return (
    <Card className="relative overflow-hidden">
      {/* Animated background gradient */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 via-transparent to-primary/5 opacity-50" />

      <CardHeader className="relative flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-base font-medium">Live Performance</CardTitle>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1.5">
            <span
              className={cn(
                "h-2 w-2 rounded-full",
                isPaused ? "bg-yellow-500" : "bg-green-500 animate-pulse"
              )}
            />
            <span className="text-xs text-muted-foreground">
              {isPaused ? "Paused" : "Live"}
            </span>
          </div>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsPaused(!isPaused)}
          >
            {isPaused ? (
              <Play className="h-3.5 w-3.5" />
            ) : (
              <Pause className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
      </CardHeader>

      <CardContent className="relative space-y-6">
        {/* Main metric - Events per second */}
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Activity className="h-4 w-4" />
            <span className="text-sm">Events Ingestion</span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-bold tabular-nums tracking-tight">
              {metrics.eventsPerSec.toLocaleString()}
            </span>
            <span className="text-lg text-muted-foreground">events/sec</span>
          </div>
        </div>

        {/* Sparkline chart */}
        <div className="h-16 w-full">
          <svg
            viewBox={`0 0 ${sparklineData.length * 4} 64`}
            className="h-full w-full"
            preserveAspectRatio="none"
          >
            {/* Gradient fill */}
            <defs>
              <linearGradient id="sparklineGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="hsl(var(--primary))" stopOpacity="0.3" />
                <stop offset="100%" stopColor="hsl(var(--primary))" stopOpacity="0" />
              </linearGradient>
            </defs>
            {/* Area fill */}
            <path
              d={`
                M 0 64
                ${sparklineData
                  .map(
                    (value, i) =>
                      `L ${i * 4} ${64 - ((value - minSparkline) / range) * 56}`
                  )
                  .join(" ")}
                L ${(sparklineData.length - 1) * 4} 64
                Z
              `}
              fill="url(#sparklineGradient)"
            />
            {/* Line */}
            {sparklineData.length > 0 && (
              <path
                d={`
                  M 0 ${64 - (((sparklineData[0] ?? 0) - minSparkline) / range) * 56}
                  ${sparklineData
                    .map(
                      (value, i) =>
                        `L ${i * 4} ${64 - ((value - minSparkline) / range) * 56}`
                    )
                    .join(" ")}
                `}
                fill="none"
                stroke="hsl(var(--primary))"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            )}
            {/* Current value dot */}
            {sparklineData.length > 0 && (
              <circle
                cx={(sparklineData.length - 1) * 4}
                cy={
                  64 -
                  (((sparklineData[sparklineData.length - 1] ?? 0) - minSparkline) /
                    range) *
                    56
                }
                r="4"
                fill="hsl(var(--primary))"
                className={cn(!isPaused && "animate-pulse")}
              />
            )}
          </svg>
        </div>

        {/* Secondary metrics */}
        <div className="grid grid-cols-2 gap-4 border-t border-border pt-4">
          <div className="space-y-1">
            <div className="flex items-center gap-1.5 text-muted-foreground">
              <Zap className="h-3.5 w-3.5" />
              <span className="text-xs">p99 Latency</span>
            </div>
            <div className="flex items-baseline gap-1">
              <span className="text-2xl font-semibold tabular-nums">
                {metrics.latencyP99}
              </span>
              <span className="text-sm text-muted-foreground">μs</span>
            </div>
          </div>
          <div className="space-y-1">
            <div className="flex items-center gap-1.5 text-muted-foreground">
              <Gauge className="h-3.5 w-3.5" />
              <span className="text-xs">Throughput</span>
            </div>
            <div className="flex items-baseline gap-1">
              <span className="text-2xl font-semibold tabular-nums">
                {metrics.throughput}
              </span>
              <span className="text-sm text-muted-foreground">%</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
