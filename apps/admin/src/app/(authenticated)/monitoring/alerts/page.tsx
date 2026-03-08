"use client";

import { useCallback, useEffect, useState } from "react";
import { Button, Card, CardContent, CardHeader, CardTitle, Badge, Input, Label } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Bell,
  Plus,
  Pencil,
  Trash2,
  X,
  ArrowLeft,
} from "lucide-react";
import Link from "next/link";
import {
  fetchAlerts,
  createAlert,
  updateAlert,
  deleteAlert,
  ALERT_METRICS,
  ALERT_OPERATORS,
  OPERATOR_SYMBOLS,
  type AlertRule,
  type CreateAlertPayload,
  type AlertOperator,
  type AlertChannel,
} from "@/lib/alerts-api";

type FormMode = "closed" | "create" | "edit";

export default function AlertsPage() {
  const [alerts, setAlerts] = useState<AlertRule[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [formMode, setFormMode] = useState<FormMode>("closed");
  const [editingId, setEditingId] = useState<string | null>(null);

  // Form state
  const [formName, setFormName] = useState("");
  const [formMetric, setFormMetric] = useState(ALERT_METRICS[0].value);
  const [formOperator, setFormOperator] = useState<AlertOperator>("gt");
  const [formThreshold, setFormThreshold] = useState("");
  const [formDuration, setFormDuration] = useState("300");
  const [formChannel, setFormChannel] = useState<AlertChannel>("email");
  const [formError, setFormError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadAlerts = useCallback(async () => {
    try {
      const data = await fetchAlerts();
      setAlerts(data);
    } catch (err) {
      console.error("Failed to load alerts:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAlerts();
  }, [loadAlerts]);

  const resetForm = () => {
    setFormName("");
    setFormMetric(ALERT_METRICS[0].value);
    setFormOperator("gt");
    setFormThreshold("");
    setFormDuration("300");
    setFormChannel("email");
    setFormError(null);
    setEditingId(null);
  };

  const openCreate = () => {
    resetForm();
    setFormMode("create");
  };

  const openEdit = (alert: AlertRule) => {
    setFormName(alert.name);
    setFormMetric(alert.metric);
    setFormOperator(alert.operator);
    setFormThreshold(String(alert.threshold));
    setFormDuration(String(alert.duration_seconds));
    setFormChannel(alert.channel);
    setEditingId(alert.id);
    setFormError(null);
    setFormMode("edit");
  };

  const closeForm = () => {
    setFormMode("closed");
    resetForm();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);

    if (!formName.trim()) {
      setFormError("Name is required.");
      return;
    }
    if (!formThreshold || isNaN(Number(formThreshold))) {
      setFormError("Threshold must be a valid number.");
      return;
    }

    const payload: CreateAlertPayload = {
      name: formName.trim(),
      metric: formMetric,
      operator: formOperator,
      threshold: Number(formThreshold),
      duration_seconds: Number(formDuration),
      channel: formChannel,
    };

    setIsSubmitting(true);
    try {
      if (formMode === "create") {
        await createAlert(payload);
      } else if (formMode === "edit" && editingId) {
        await updateAlert(editingId, payload);
      }
      closeForm();
      await loadAlerts();
    } catch (err) {
      setFormError(err instanceof Error ? err.message : "Failed to save alert.");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteAlert(id);
      setAlerts((prev) => prev.filter((a) => a.id !== id));
    } catch (err) {
      console.error("Failed to delete alert:", err);
    }
  };

  const metricLabel = (metric: string) =>
    ALERT_METRICS.find((m) => m.value === metric)?.label ?? metric;

  return (
    <div className="space-y-6" data-testid="alerts-page">
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
            <h1 className="text-3xl font-bold tracking-tight">Alert Rules</h1>
            <p className="text-muted-foreground">
              Configure alerts for platform metrics.
            </p>
          </div>
        </div>
        <Button onClick={openCreate} data-testid="create-alert-btn">
          <Plus className="mr-2 h-4 w-4" />
          Create Alert
        </Button>
      </div>

      {/* Create / Edit form */}
      {formMode !== "closed" && (
        <Card data-testid="alert-form">
          <CardHeader className="flex flex-row items-center justify-between pb-4">
            <CardTitle>
              {formMode === "create" ? "Create Alert Rule" : "Edit Alert Rule"}
            </CardTitle>
            <button
              type="button"
              onClick={closeForm}
              className="rounded-md p-1 text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {/* Name */}
                <div className="space-y-2">
                  <Label htmlFor="alert-name">Name</Label>
                  <Input
                    id="alert-name"
                    data-testid="alert-name-input"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    placeholder="High error rate"
                  />
                </div>

                {/* Metric */}
                <div className="space-y-2">
                  <Label htmlFor="alert-metric">Metric</Label>
                  <select
                    id="alert-metric"
                    data-testid="alert-metric-select"
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

                {/* Operator */}
                <div className="space-y-2">
                  <Label htmlFor="alert-operator">Operator</Label>
                  <select
                    id="alert-operator"
                    data-testid="alert-operator-select"
                    value={formOperator}
                    onChange={(e) =>
                      setFormOperator(e.target.value as AlertOperator)
                    }
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    {ALERT_OPERATORS.map((op) => (
                      <option key={op.value} value={op.value}>
                        {op.label}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Threshold */}
                <div className="space-y-2">
                  <Label htmlFor="alert-threshold">Threshold</Label>
                  <Input
                    id="alert-threshold"
                    data-testid="alert-threshold-input"
                    type="number"
                    step="any"
                    value={formThreshold}
                    onChange={(e) => setFormThreshold(e.target.value)}
                    placeholder="5"
                  />
                </div>

                {/* Duration */}
                <div className="space-y-2">
                  <Label htmlFor="alert-duration">Duration (seconds)</Label>
                  <Input
                    id="alert-duration"
                    data-testid="alert-duration-input"
                    type="number"
                    value={formDuration}
                    onChange={(e) => setFormDuration(e.target.value)}
                    placeholder="300"
                  />
                </div>

                {/* Channel */}
                <div className="space-y-2">
                  <Label htmlFor="alert-channel">Channel</Label>
                  <select
                    id="alert-channel"
                    data-testid="alert-channel-select"
                    value={formChannel}
                    onChange={(e) =>
                      setFormChannel(e.target.value as AlertChannel)
                    }
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  >
                    <option value="email">Email</option>
                    <option value="slack">Slack</option>
                  </select>
                </div>
              </div>

              {formError && (
                <p className="text-sm text-red-500" data-testid="alert-form-error">
                  {formError}
                </p>
              )}

              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={closeForm}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={isSubmitting}
                  data-testid="alert-submit-btn"
                >
                  {isSubmitting
                    ? "Saving..."
                    : formMode === "create"
                      ? "Create"
                      : "Update"}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {/* Alerts table */}
      <Card>
        <CardContent className="p-0">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
            </div>
          ) : alerts.length === 0 ? (
            <div
              className="flex flex-col items-center justify-center py-12 text-center"
              data-testid="alerts-empty"
            >
              <Bell className="mb-3 h-10 w-10 text-muted-foreground/50" />
              <p className="text-sm text-muted-foreground">
                No alert rules configured yet.
              </p>
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table
                className="w-full text-sm"
                data-testid="alerts-table"
              >
                <thead>
                  <tr className="border-b border-border text-left">
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Name
                    </th>
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Condition
                    </th>
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Duration
                    </th>
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Channel
                    </th>
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Status
                    </th>
                    <th className="px-4 py-3 font-medium text-muted-foreground">
                      Actions
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {alerts.map((alert) => (
                    <tr
                      key={alert.id}
                      className="border-b border-border last:border-0"
                      data-testid={`alert-row-${alert.id}`}
                    >
                      <td className="px-4 py-3 font-medium">{alert.name}</td>
                      <td className="px-4 py-3 text-muted-foreground">
                        {metricLabel(alert.metric)}{" "}
                        {OPERATOR_SYMBOLS[alert.operator]} {alert.threshold}
                      </td>
                      <td className="px-4 py-3 text-muted-foreground">
                        {alert.duration_seconds}s
                      </td>
                      <td className="px-4 py-3">
                        <Badge variant="secondary">{alert.channel}</Badge>
                      </td>
                      <td className="px-4 py-3">
                        <Badge
                          variant={alert.enabled ? "default" : "secondary"}
                          className={cn(
                            alert.enabled
                              ? "bg-green-500/10 text-green-600 hover:bg-green-500/20"
                              : ""
                          )}
                        >
                          {alert.enabled ? "Active" : "Disabled"}
                        </Badge>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => openEdit(alert)}
                            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
                            aria-label={`Edit ${alert.name}`}
                            data-testid={`edit-alert-${alert.id}`}
                          >
                            <Pencil className="h-4 w-4" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleDelete(alert.id)}
                            className="rounded-md p-1.5 text-muted-foreground hover:bg-red-500/10 hover:text-red-500 transition-colors"
                            aria-label={`Delete ${alert.name}`}
                            data-testid={`delete-alert-${alert.id}`}
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
