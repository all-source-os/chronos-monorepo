"use client";

import {
  Badge,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { Activity, Stethoscope } from "lucide-react";
import Link from "next/link";
import { HealthChip } from "@/components/fleet/health-chip";
import { UsageBar } from "@/components/tenants/usage-bar";
import type { TenantHealth } from "@/lib/fleet-api";

/**
 * Inline health panel for the tenant 360 (Pillar A — Health section). It renders
 * the EXACT shape returned by GET /api/v1/admin/fleet/health/:id, reusing the
 * fleet signal/health-chip vocabulary verbatim so the panel reads identically to
 * the fleet drill-down (/fleet/[id]). It performs NO scoring — it displays the
 * tiers/values/sources the Control Plane returns.
 *
 * Presentation-only: the parent page fetches the health once (via
 * fetchTenantHealth from lib/fleet-api) and passes it here AND to the Operations
 * panel, so health is fetched once and drives both the display and the
 * health-driven operation prominence.
 */

interface TenantHealthPanelProps {
  health: TenantHealth | null;
  /** Deep-link to the full fleet drill-down for the same tenant. */
  tenantId: string;
  /** True when the fleet endpoint was unreachable and `health` is null. */
  unavailable?: boolean;
}

export function TenantHealthPanel({ health, tenantId, unavailable }: TenantHealthPanelProps) {
  const sub = health?.subscription;
  // Guard the list so a wrapped/odd/empty payload never crashes the .map (§6).
  const signals = Array.isArray(health?.signals) ? health.signals : [];

  return (
    <Card data-testid="tenant-health-panel">
      <CardHeader>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-5 w-5" />
              Health
              {health && <HealthChip tier={health.tier} />}
            </CardTitle>
            <CardDescription>
              Every signal, its observed value, the tier it triggered, and the backend it was read
              from — from <span className="font-mono">/api/v1/admin/fleet/health/{tenantId}</span>.
            </CardDescription>
          </div>
          <Link
            href={`/fleet/${tenantId}`}
            className="text-sm text-muted-foreground underline hover:text-foreground"
            data-testid="health-fleet-link"
          >
            Open fleet drill-down
          </Link>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {unavailable || !health ? (
          <div
            className="flex items-center gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-600"
            data-testid="health-unavailable"
          >
            <Stethoscope className="h-4 w-4 shrink-0" />
            <span>
              Health assessment unavailable — the Control Plane fleet-health endpoint did not
              answer. Signals will populate once it is reachable.
            </span>
          </div>
        ) : (
          <>
            {signals.length === 0 ? (
              <p
                className="py-4 text-center text-sm text-muted-foreground"
                data-testid="health-no-signals"
              >
                No health signals reported.
              </p>
            ) : (
              <Table data-testid="health-signals-table">
                <TableHeader>
                  <TableRow>
                    <TableHead>Signal</TableHead>
                    <TableHead>Value</TableHead>
                    <TableHead>Tier</TableHead>
                    <TableHead>Source</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {signals.map((s) => (
                    <TableRow key={s.signal} data-testid={`health-signal-${s.signal}`}>
                      <TableCell className="font-mono text-sm">{s.signal}</TableCell>
                      <TableCell className="font-medium">{s.value}</TableCell>
                      <TableCell>
                        <HealthChip tier={s.tier} />
                      </TableCell>
                      <TableCell className="text-sm text-muted-foreground">{s.source}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}

            {sub && (
              <div className="space-y-3 rounded-lg border p-4" data-testid="health-subscription">
                <p className="text-sm font-medium">Subscription signal</p>
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Tier</span>
                    <span className="text-sm font-medium capitalize">{sub.tier || "--"}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Status</span>
                    <Badge
                      variant={
                        sub.status === "active" || sub.status === "on_trial"
                          ? "default"
                          : sub.status === "past_due"
                            ? "secondary"
                            : "destructive"
                      }
                      data-testid="health-sub-status"
                    >
                      {sub.status || "unknown"}
                    </Badge>
                  </div>
                </div>
                {sub.events_quota !== undefined && (
                  <UsageBar
                    label="Events quota"
                    current={sub.events_used ?? 0}
                    limit={sub.events_quota ?? 0}
                    testId="health-quota-bar"
                  />
                )}
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
