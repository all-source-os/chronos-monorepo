"use client";

import { cn } from "@allsource/ui/utils";

interface UsageBarProps {
  label: string;
  current: number;
  limit: number;
  unit?: string;
  testId?: string;
}

export function UsageBar({ label, current, limit, unit = "", testId }: UsageBarProps) {
  const pct = limit > 0 ? Math.min((current / limit) * 100, 100) : 0;
  const status = pct >= 90 ? "critical" : pct >= 70 ? "warning" : "healthy";

  return (
    <div className="space-y-1.5" data-testid={testId}>
      <div className="flex items-center justify-between text-sm">
        <span className="font-medium">{label}</span>
        <span className="text-muted-foreground tabular-nums">
          {current.toLocaleString()}{unit} / {limit.toLocaleString()}{unit} ({pct.toFixed(1)}%)
        </span>
      </div>
      <div className="h-2 w-full rounded-full bg-muted">
        <div
          className={cn(
            "h-full rounded-full transition-all",
            status === "healthy" && "bg-green-500",
            status === "warning" && "bg-yellow-500",
            status === "critical" && "bg-red-500"
          )}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
