"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, ArrowRight } from "lucide-react";
import Link from "next/link";
import { useAuthStore } from "@/lib/stores/auth-store";

interface UsageBarProps {
  label: string;
  used: number;
  quota: number;
  unit?: string;
  hint?: string;
}

function UsageBar({ label, used, quota, unit = "", hint }: UsageBarProps) {
  const percentage = quota > 0 ? (used / quota) * 100 : 0;
  const isWarning = percentage >= 80;
  const isCritical = percentage >= 95;

  const formatNumber = (num: number) => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-sm">
        <span className="font-medium">{label}</span>
        <span className="text-muted-foreground">
          {formatNumber(used)}
          {unit} / {formatNumber(quota)}
          {unit}
        </span>
      </div>
      <div className="relative h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className={cn(
            "h-full rounded-full transition-all duration-500",
            isCritical ? "bg-destructive" : isWarning ? "bg-yellow-500" : "bg-primary"
          )}
          style={{ width: `${Math.min(percentage, 100)}%` }}
        />
      </div>
      {isWarning && (
        <div
          className={cn(
            "flex items-center gap-1 text-xs",
            isCritical ? "text-destructive" : "text-yellow-600 dark:text-yellow-400"
          )}
        >
          <AlertTriangle className="h-3 w-3" />
          {isCritical
            ? "Approaching limit - upgrade to continue"
            : "Usage is high - consider upgrading"}
        </div>
      )}
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export function UsageProgress() {
  const { tenant } = useAuthStore();

  const eventsUsed = tenant?.events_used || 0;
  const eventsQuota = tenant?.events_quota || 100000;
  const queriesUsed = tenant?.queries_used || 0;
  const queriesQuota = tenant?.queries_quota || 10000;

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-base font-medium">Usage This Month</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link href="/dashboard/billing" className="flex items-center gap-1">
            Manage plan
            <ArrowRight className="h-3.5 w-3.5" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent className="space-y-6">
        <UsageBar label="Events" used={eventsUsed} quota={eventsQuota} />
        <UsageBar
          label="Queries"
          used={queriesUsed}
          quota={queriesQuota}
          hint="Counts reads routed through the API gateway; direct-to-Core reads aren't metered yet."
        />
      </CardContent>
    </Card>
  );
}
