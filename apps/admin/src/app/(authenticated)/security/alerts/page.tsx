"use client";

import { useCallback, useEffect, useState } from "react";
import {
  Badge,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Skeleton,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertTriangle,
  ArrowLeft,
  Bell,
  Filter,
  KeyRound,
  MapPin,
  ShieldAlert,
  Zap,
} from "lucide-react";
import Link from "next/link";
import {
  fetchSuspiciousActivity,
  ACTIVITY_TYPE_LABELS,
  SEVERITY_CONFIG,
  type AlertSeverity,
  type SuspiciousActivityAlert,
  type SuspiciousActivityType,
} from "@/lib/suspicious-activity-api";

// ---------------------------------------------------------------------------
// Type icon mapping
// ---------------------------------------------------------------------------

const TYPE_ICONS: Record<SuspiciousActivityType, React.ElementType> = {
  excessive_failed_auth: ShieldAlert,
  unusual_query_pattern: Zap,
  rate_limit_exceeded: AlertTriangle,
  token_abuse: KeyRound,
  geo_anomaly: MapPin,
};

// ---------------------------------------------------------------------------
// Alert Card
// ---------------------------------------------------------------------------

function AlertCard({ alert }: { alert: SuspiciousActivityAlert }) {
  const severity = SEVERITY_CONFIG[alert.severity];
  const Icon = TYPE_ICONS[alert.type] ?? AlertTriangle;

  // Contextual link: token_abuse -> token audit, otherwise -> tenant detail
  const href =
    alert.type === "token_abuse"
      ? "/security"
      : `/tenants/${alert.tenant_id}`;

  return (
    <Link href={href} data-testid={`alert-card-${alert.id}`}>
      <Card
        className={cn(
          "border transition-colors hover:shadow-md cursor-pointer",
          severity.className
        )}
      >
        <CardHeader className="flex flex-row items-start gap-3 pb-2">
          <div
            className={cn(
              "mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-lg",
              severity.className
            )}
          >
            <Icon className="h-5 w-5" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center justify-between gap-2">
              <CardTitle className="text-sm font-semibold truncate">
                {ACTIVITY_TYPE_LABELS[alert.type] ?? alert.type}
              </CardTitle>
              <Badge
                variant="outline"
                className={cn("shrink-0 text-xs", severity.className)}
                data-testid={`alert-severity-${alert.id}`}
              >
                <span
                  className={cn(
                    "mr-1.5 inline-block h-1.5 w-1.5 rounded-full",
                    severity.dotClassName
                  )}
                />
                {severity.label}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              Tenant:{" "}
              <span className="font-medium">{alert.tenant_name}</span>
            </p>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
          <p className="text-sm">{alert.description}</p>
          <p className="mt-2 text-xs text-muted-foreground">
            {new Date(alert.timestamp).toLocaleString()}
          </p>
        </CardContent>
      </Card>
    </Link>
  );
}

// ---------------------------------------------------------------------------
// Severity Filter
// ---------------------------------------------------------------------------

const SEVERITY_OPTIONS: { value: AlertSeverity | "all"; label: string }[] = [
  { value: "all", label: "All Severities" },
  { value: "high", label: "High" },
  { value: "medium", label: "Medium" },
  { value: "low", label: "Low" },
];

const TYPE_OPTIONS: { value: SuspiciousActivityType | "all"; label: string }[] =
  [
    { value: "all", label: "All Types" },
    { value: "excessive_failed_auth", label: "Excessive Failed Auth" },
    { value: "unusual_query_pattern", label: "Unusual Query Pattern" },
    { value: "rate_limit_exceeded", label: "Rate Limit Exceeded" },
    { value: "token_abuse", label: "Token Abuse" },
    { value: "geo_anomaly", label: "Geographic Anomaly" },
  ];

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

export default function SuspiciousActivityAlertsPage() {
  const [alerts, setAlerts] = useState<SuspiciousActivityAlert[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [severityFilter, setSeverityFilter] = useState<
    AlertSeverity | "all"
  >("all");
  const [typeFilter, setTypeFilter] = useState<
    SuspiciousActivityType | "all"
  >("all");

  const loadAlerts = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchSuspiciousActivity({
        severity: severityFilter === "all" ? undefined : severityFilter,
        type: typeFilter === "all" ? undefined : typeFilter,
      });
      setAlerts(data.alerts);
    } catch (err) {
      console.error("Failed to load suspicious activity alerts:", err);
    } finally {
      setIsLoading(false);
    }
  }, [severityFilter, typeFilter]);

  useEffect(() => {
    loadAlerts();
  }, [loadAlerts]);

  // Auto-refresh every 60 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      loadAlerts();
    }, 60_000);
    return () => clearInterval(interval);
  }, [loadAlerts]);

  const highCount = alerts.filter((a) => a.severity === "high").length;
  const mediumCount = alerts.filter((a) => a.severity === "medium").length;
  const lowCount = alerts.filter((a) => a.severity === "low").length;

  return (
    <div className="space-y-6" data-testid="suspicious-activity-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link
            href="/security"
            className="rounded-md p-2 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
          >
            <ArrowLeft className="h-4 w-4" />
          </Link>
          <div>
            <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
              <Bell className="h-8 w-8" />
              Suspicious Activity
            </h1>
            <p className="text-muted-foreground">
              Security alerts for anomalous tenant behavior.
            </p>
          </div>
        </div>
      </div>

      {/* Summary badges */}
      <div className="flex items-center gap-3" data-testid="alert-summary">
        <Badge
          variant="outline"
          className={cn(SEVERITY_CONFIG.high.className, "text-sm px-3 py-1")}
          data-testid="summary-high"
        >
          {highCount} High
        </Badge>
        <Badge
          variant="outline"
          className={cn(
            SEVERITY_CONFIG.medium.className,
            "text-sm px-3 py-1"
          )}
          data-testid="summary-medium"
        >
          {mediumCount} Medium
        </Badge>
        <Badge
          variant="outline"
          className={cn(SEVERITY_CONFIG.low.className, "text-sm px-3 py-1")}
          data-testid="summary-low"
        >
          {lowCount} Low
        </Badge>
      </div>

      {/* Filters */}
      <div
        className="flex flex-wrap items-center gap-3"
        data-testid="alert-filters"
      >
        <Filter className="h-4 w-4 text-muted-foreground" />
        <select
          value={severityFilter}
          onChange={(e) =>
            setSeverityFilter(e.target.value as AlertSeverity | "all")
          }
          className="flex h-9 rounded-md border border-input bg-background px-3 py-1 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          data-testid="severity-filter"
        >
          {SEVERITY_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          value={typeFilter}
          onChange={(e) =>
            setTypeFilter(e.target.value as SuspiciousActivityType | "all")
          }
          className="flex h-9 rounded-md border border-input bg-background px-3 py-1 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          data-testid="type-filter"
        >
          {TYPE_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>

      {/* Alert cards */}
      {isLoading ? (
        <div className="grid gap-4 md:grid-cols-2" data-testid="alerts-skeleton">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-36 w-full rounded-xl" />
          ))}
        </div>
      ) : alerts.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Bell className="mb-3 h-10 w-10 text-muted-foreground/50" />
            <p
              className="text-sm text-muted-foreground"
              data-testid="alerts-empty"
            >
              No suspicious activity alerts found.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div
          className="grid gap-4 md:grid-cols-2"
          data-testid="alerts-grid"
        >
          {alerts.map((alert) => (
            <AlertCard key={alert.id} alert={alert} />
          ))}
        </div>
      )}
    </div>
  );
}
