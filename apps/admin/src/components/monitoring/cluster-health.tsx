"use client";

import { Card, CardContent, CardDescription, CardHeader, CardTitle, Badge } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Server } from "lucide-react";
import type { ClusterMember } from "@/lib/metrics-api";
import { formatUptime } from "@/lib/metrics-api";

interface ClusterHealthProps {
  members: ClusterMember[];
  isLoading?: boolean;
}

export function ClusterHealth({ members, isLoading }: ClusterHealthProps) {
  if (isLoading) {
    return (
      <Card data-testid="cluster-health">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Server className="h-5 w-5" />
            Cluster Health
          </CardTitle>
          <CardDescription>Core leader and follower status</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card data-testid="cluster-health">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Server className="h-5 w-5" />
          Cluster Health
        </CardTitle>
        <CardDescription>Core leader and follower status</CardDescription>
      </CardHeader>
      <CardContent>
        {members.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No cluster members found.
          </p>
        ) : (
          <div className="space-y-3">
            {members.map((member) => (
              <div
                key={member.id}
                className="flex items-center justify-between rounded-lg border p-4"
                data-testid={`cluster-member-${member.id}`}
              >
                <div className="flex items-center gap-3">
                  <div
                    className={cn(
                      "h-3 w-3 rounded-full",
                      member.status === "healthy" && "bg-green-500",
                      member.status === "degraded" && "bg-yellow-500",
                      member.status === "unreachable" && "bg-red-500"
                    )}
                  />
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{member.id}</span>
                      <Badge
                        variant={member.role === "leader" ? "default" : "secondary"}
                      >
                        {member.role}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {member.address}
                    </p>
                  </div>
                </div>
                <div className="text-right text-sm">
                  <p
                    className={cn(
                      "font-medium capitalize",
                      member.status === "healthy" && "text-green-500",
                      member.status === "degraded" && "text-yellow-500",
                      member.status === "unreachable" && "text-red-500"
                    )}
                  >
                    {member.status}
                  </p>
                  {member.lag_ms !== undefined && member.role === "follower" && (
                    <p className="text-xs text-muted-foreground">
                      Lag: {member.lag_ms}ms
                    </p>
                  )}
                  {member.uptime_seconds !== undefined && (
                    <p className="text-xs text-muted-foreground">
                      Up: {formatUptime(member.uptime_seconds)}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
