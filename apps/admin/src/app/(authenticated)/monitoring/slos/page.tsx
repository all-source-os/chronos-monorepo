"use client";

import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Target, Plus, Trash2, X, ArrowLeft } from "lucide-react";
import Link from "next/link";
import {
  fetchSLOs,
  createSLO,
  deleteSLO,
  ALERT_METRICS,
  type SLO,
  type CreateSLOPayload,
} from "@/lib/alerts-api";

// ---------------------------------------------------------------------------
// Sparkline — lightweight inline SVG chart
// ---------------------------------------------------------------------------

function Sparkline({
  data,
  color,
  width = 120,
  height = 32,
}: {
  data: number[];
  color: string;
  width?: number;
  height?: number;
}) {
  if (data.length < 2) return null;

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const points = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * width;
      const y = height - ((v - min) / range) * (height - 4) - 2;
      return `${x},${y}`;
    })
    .join(" ");

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className="inline-block"
      data-testid="sparkline"
    >
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Compliance gauge — circular progress indicator
// ---------------------------------------------------------------------------

function ComplianceGauge({
  value,
  size = 80,
}: {
  value: number;
  size?: number;
}) {
  const radius = (size - 8) / 2;
  const circumference = 2 * Math.PI * radius;
  const progress = Math.min(Math.max(value, 0), 100);
  const offset = circumference - (progress / 100) * circumference;

  const color =
    value >= 99
      ? "text-green-500"
      : value >= 95
        ? "text-yellow-500"
        : "text-red-500";

  return (
    <div
      className="relative flex items-center justify-center"
      data-testid="compliance-gauge"
    >
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth={4}
          className="text-muted/30"
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth={4}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          className={color}
        />
      </svg>
      <span
        className={cn(
          "absolute text-sm font-bold tabular-nums",
          color
        )}
      >
        {value.toFixed(1)}%
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SLOs page
// ---------------------------------------------------------------------------

export default function SLOsPage() {
  const [slos, setSlos] = useState<SLO[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);

  // Form state
  const [formName, setFormName] = useState("");
  const [formMetric, setFormMetric] = useState(ALERT_METRICS[0].value);
  const [formTarget, setFormTarget] = useState("99.9");
  const [formWindow, setFormWindow] = useState("30");
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadSLOs = useCallback(async () => {
    try {
      const data = await fetchSLOs();
      setSlos(data);
    } catch (err) {
      console.error("Failed to load SLOs:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSLOs();
  }, [loadSLOs]);

  const resetForm = () => {
    setFormName("");
    setFormMetric(ALERT_METRICS[0].value);
    setFormTarget("99.9");
    setFormWindow("30");
    setFormError(null);
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

    if (!formName.trim()) {
      setFormError("Name is required.");
      return;
    }

    const target = Number(formTarget);
    if (isNaN(target) || target <= 0 || target > 100) {
      setFormError("Target must be between 0 and 100.");
      return;
    }

    const payload: CreateSLOPayload = {
      name: formName.trim(),
      metric: formMetric,
      target_percent: target,
      window_days: Number(formWindow),
    };

    setIsSubmitting(true);
    try {
      await createSLO(payload);
      setShowForm(false);
      resetForm();
      await loadSLOs();
    } catch (err) {
      setFormError(
        err instanceof Error ? err.message : "Failed to create SLO."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteSLO(id);
      setSlos((prev) => prev.filter((s) => s.id !== id));
    } catch (err) {
      console.error("Failed to delete SLO:", err);
    }
  };

  const complianceStatus = (value: number) =>
    value >= 99 ? "healthy" : value >= 95 ? "warning" : "critical";

  const gaugeColor = (value: number) =>
    value >= 99
      ? "hsl(142, 76%, 36%)"
      : value >= 95
        ? "hsl(45, 93%, 47%)"
        : "hsl(0, 84%, 60%)";

  const metricLabel = (metric: string) =>
    ALERT_METRICS.find((m) => m.value === metric)?.label ?? metric;

  return (
    <div className="space-y-6" data-testid="slos-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Link
            href="/monitoring"
            className="rounded-md p-2 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
          >
            <ArrowLeft className="h-4 w-4" />
          </Link>
          <div>
            <h1 className="text-3xl font-bold tracking-tight">
              Service Level Objectives
            </h1>
            <p className="text-muted-foreground">
              Track SLO compliance and error budgets.
            </p>
          </div>
        </div>
        <Button
          onClick={() => {
            resetForm();
            setShowForm(true);
          }}
          data-testid="create-slo-btn"
        >
          <Plus className="mr-2 h-4 w-4" />
          Create SLO
        </Button>
      </div>

      {/* Create form */}
      {showForm && (
        <Card data-testid="slo-form">
          <CardHeader className="flex flex-row items-center justify-between pb-4">
            <CardTitle>Create SLO</CardTitle>
            <button
              type="button"
              onClick={() => setShowForm(false)}
              className="rounded-md p-1 text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                <div className="space-y-2">
                  <Label htmlFor="slo-name">Name</Label>
                  <Input
                    id="slo-name"
                    data-testid="slo-name-input"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    placeholder="API Availability"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="slo-metric">Metric</Label>
                  <select
                    id="slo-metric"
                    data-testid="slo-metric-select"
                    value={formMetric}
                    onChange={(e) => setFormMetric(e.target.value)}
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    {ALERT_METRICS.map((m) => (
                      <option key={m.value} value={m.value}>
                        {m.label}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="slo-target">Target (%)</Label>
                  <Input
                    id="slo-target"
                    data-testid="slo-target-input"
                    type="number"
                    step="0.01"
                    min="0"
                    max="100"
                    value={formTarget}
                    onChange={(e) => setFormTarget(e.target.value)}
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="slo-window">Window (days)</Label>
                  <Input
                    id="slo-window"
                    data-testid="slo-window-input"
                    type="number"
                    min="1"
                    value={formWindow}
                    onChange={(e) => setFormWindow(e.target.value)}
                  />
                </div>
              </div>

              {formError && (
                <p
                  className="text-sm text-red-500"
                  data-testid="slo-form-error"
                >
                  {formError}
                </p>
              )}

              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setShowForm(false)}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={isSubmitting}
                  data-testid="slo-submit-btn"
                >
                  {isSubmitting ? "Creating..." : "Create"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {/* SLO cards */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
        </div>
      ) : slos.length === 0 ? (
        <div
          className="flex flex-col items-center justify-center py-12 text-center"
          data-testid="slos-empty"
        >
          <Target className="mb-3 h-10 w-10 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">
            No SLOs configured yet.
          </p>
        </div>
      ) : (
        <div
          className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3"
          data-testid="slo-cards"
        >
          {slos.map((slo) => {
            const status = complianceStatus(slo.current_compliance);
            return (
              <Card
                key={slo.id}
                data-testid={`slo-card-${slo.id}`}
                className={cn(
                  "relative overflow-hidden",
                  status === "critical" && "border-red-500/30",
                  status === "warning" && "border-yellow-500/30"
                )}
              >
                <CardHeader className="flex flex-row items-start justify-between pb-2">
                  <div>
                    <CardTitle className="text-base">{slo.name}</CardTitle>
                    <p className="text-xs text-muted-foreground mt-1">
                      {metricLabel(slo.metric)} &middot; {slo.window_days}d
                      window &middot; Target: {slo.target_percent}%
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleDelete(slo.id)}
                    className="rounded-md p-1.5 text-muted-foreground hover:bg-red-500/10 hover:text-red-500 transition-colors"
                    aria-label={`Delete ${slo.name}`}
                    data-testid={`delete-slo-${slo.id}`}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="space-y-2">
                      <div>
                        <p className="text-xs text-muted-foreground">
                          Current Compliance
                        </p>
                        <p
                          className={cn(
                            "text-xl font-bold tabular-nums",
                            status === "healthy" && "text-green-500",
                            status === "warning" && "text-yellow-500",
                            status === "critical" && "text-red-500"
                          )}
                          data-testid="slo-compliance"
                        >
                          {slo.current_compliance.toFixed(2)}%
                        </p>
                      </div>
                      <div>
                        <p className="text-xs text-muted-foreground">
                          Error Budget Remaining
                        </p>
                        <p
                          className="text-sm font-medium tabular-nums"
                          data-testid="slo-error-budget"
                        >
                          {slo.error_budget_remaining.toFixed(1)}%
                        </p>
                      </div>
                      {slo.trend && slo.trend.length > 1 && (
                        <div>
                          <p className="text-xs text-muted-foreground mb-1">
                            Trend
                          </p>
                          <Sparkline
                            data={slo.trend}
                            color={gaugeColor(slo.current_compliance)}
                          />
                        </div>
                      )}
                    </div>
                    <ComplianceGauge value={slo.current_compliance} />
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
