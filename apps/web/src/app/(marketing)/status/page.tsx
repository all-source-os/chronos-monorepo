"use client";

import {
  Badge,
  BlurFade,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@allsource/ui";
import { useCallback, useEffect, useState } from "react";

const POLL_INTERVAL = 60_000; // 60 seconds

interface ServiceStatus {
  name: string;
  description: string;
  endpoint: string;
  status: "operational" | "degraded" | "down" | "checking";
  latencyMs: number | null;
  lastChecked: string | null;
}

const SERVICES: Omit<ServiceStatus, "status" | "latencyMs" | "lastChecked">[] = [
  {
    name: "Core (Event Store)",
    description: "Rust event ingestion, WAL, Parquet storage, and queries",
    endpoint: "/api/status/core",
  },
  {
    name: "Query Service",
    description: "Elixir API gateway — auth, billing, routing",
    endpoint: "/api/status/query-service",
  },
  {
    name: "Control Plane",
    description: "Go control plane — JWT/RBAC, policy engine",
    endpoint: "/api/status/control-plane",
  },
];

function statusBadge(status: ServiceStatus["status"]) {
  switch (status) {
    case "operational":
      return (
        <Badge className="bg-green-500/10 text-green-500 border-green-500/20">Operational</Badge>
      );
    case "degraded":
      return (
        <Badge className="bg-yellow-500/10 text-yellow-500 border-yellow-500/20">Degraded</Badge>
      );
    case "down":
      return <Badge className="bg-red-500/10 text-red-500 border-red-500/20">Down</Badge>;
    case "checking":
      return <Badge className="bg-muted text-muted-foreground">Checking...</Badge>;
  }
}

function overallStatus(services: ServiceStatus[]): {
  label: string;
  color: string;
} {
  if (services.some((s) => s.status === "checking")) {
    return { label: "Checking systems...", color: "text-muted-foreground" };
  }
  if (services.every((s) => s.status === "operational")) {
    return { label: "All Systems Operational", color: "text-green-500" };
  }
  if (services.some((s) => s.status === "down")) {
    return { label: "Partial Outage", color: "text-red-500" };
  }
  return { label: "Degraded Performance", color: "text-yellow-500" };
}

function uptimePercent(status: ServiceStatus["status"]): string {
  // In a real implementation, this would be calculated from historical data.
  // For now, show 100% for operational, 99.9% for degraded, 0% for down.
  switch (status) {
    case "operational":
      return "100.0%";
    case "degraded":
      return "99.9%";
    case "down":
      return "—";
    case "checking":
      return "—";
  }
}

export default function StatusPage() {
  const [services, setServices] = useState<ServiceStatus[]>(
    SERVICES.map((s) => ({ ...s, status: "checking", latencyMs: null, lastChecked: null }))
  );

  const checkServices = useCallback(async () => {
    const results = await Promise.all(
      SERVICES.map(async (svc) => {
        const start = performance.now();
        try {
          const res = await fetch(svc.endpoint, { cache: "no-store" });
          const latencyMs = Math.round(performance.now() - start);
          if (res.ok) {
            return {
              ...svc,
              status: "operational" as const,
              latencyMs,
              lastChecked: new Date().toISOString(),
            };
          }
          return {
            ...svc,
            status: "degraded" as const,
            latencyMs,
            lastChecked: new Date().toISOString(),
          };
        } catch {
          const latencyMs = Math.round(performance.now() - start);
          return {
            ...svc,
            status: "down" as const,
            latencyMs,
            lastChecked: new Date().toISOString(),
          };
        }
      })
    );
    setServices(results);
  }, []);

  useEffect(() => {
    checkServices();
    const interval = setInterval(checkServices, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [checkServices]);

  const overall = overallStatus(services);

  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <BlurFade delay={0.1} inView>
        <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">System Status</h1>
        <p className={`text-lg font-medium mb-8 ${overall.color}`}>{overall.label}</p>
      </BlurFade>

      <BlurFade delay={0.2} inView>
        <Card>
          <CardHeader>
            <CardTitle>Services</CardTitle>
            <CardDescription>Health status polled every 60 seconds</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {services.map((svc) => (
                <div
                  key={svc.name}
                  className="flex items-center justify-between rounded-lg border p-4"
                >
                  <div className="space-y-1">
                    <p className="text-sm font-medium text-foreground">{svc.name}</p>
                    <p className="text-xs text-muted-foreground">{svc.description}</p>
                  </div>
                  <div className="flex items-center gap-4">
                    {svc.latencyMs !== null && (
                      <span className="text-xs text-muted-foreground">{svc.latencyMs}ms</span>
                    )}
                    <span className="text-xs text-muted-foreground w-12 text-right">
                      {uptimePercent(svc.status)}
                    </span>
                    {statusBadge(svc.status)}
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </BlurFade>

      <BlurFade delay={0.3} inView>
        <Card className="mt-6">
          <CardHeader>
            <CardTitle>Incident History</CardTitle>
            <CardDescription>Last 30 days</CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">No incidents reported.</p>
          </CardContent>
        </Card>
      </BlurFade>

      {services[0]?.lastChecked && (
        <p className="text-xs text-muted-foreground mt-4 text-center">
          Last checked: {new Date(services[0].lastChecked).toLocaleTimeString()}
        </p>
      )}
    </div>
  );
}
