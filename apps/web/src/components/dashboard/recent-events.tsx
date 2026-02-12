"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Activity, ArrowRight, Clock } from "lucide-react";
import Link from "next/link";
import { useEffect, useState } from "react";

interface RecentEvent {
  id: string;
  entity_id: string;
  event_type: string;
  timestamp: string;
}

export function RecentEvents() {
  const [events, setEvents] = useState<RecentEvent[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Simulate fetching recent events
  useEffect(() => {
    const sampleEvents: RecentEvent[] = [
      {
        id: "evt_1",
        entity_id: "user-123",
        event_type: "user.signed_up",
        timestamp: new Date(Date.now() - 1000 * 30).toISOString(),
      },
      {
        id: "evt_2",
        entity_id: "order-456",
        event_type: "order.placed",
        timestamp: new Date(Date.now() - 1000 * 60 * 2).toISOString(),
      },
      {
        id: "evt_3",
        entity_id: "payment-789",
        event_type: "payment.completed",
        timestamp: new Date(Date.now() - 1000 * 60 * 5).toISOString(),
      },
      {
        id: "evt_4",
        entity_id: "user-234",
        event_type: "user.profile_updated",
        timestamp: new Date(Date.now() - 1000 * 60 * 10).toISOString(),
      },
      {
        id: "evt_5",
        entity_id: "inventory-567",
        event_type: "inventory.updated",
        timestamp: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
      },
    ];

    setTimeout(() => {
      setEvents(sampleEvents);
      setIsLoading(false);
    }, 500);
  }, []);

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);

    if (diffSec < 60) return `${diffSec}s ago`;
    if (diffMin < 60) return `${diffMin}m ago`;
    return date.toLocaleTimeString();
  };

  const getEventTypeColor = (eventType: string) => {
    if (eventType.includes("user")) return "bg-blue-500";
    if (eventType.includes("order")) return "bg-green-500";
    if (eventType.includes("payment")) return "bg-purple-500";
    if (eventType.includes("inventory")) return "bg-orange-500";
    return "bg-gray-500";
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-base font-medium">Recent Events</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link href="/dashboard/events" className="flex items-center gap-1">
            View all
            <ArrowRight className="h-3.5 w-3.5" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="flex items-center gap-3">
                <div className="h-2 w-2 rounded-full bg-muted animate-pulse" />
                <div className="flex-1 space-y-1">
                  <div className="h-4 w-24 rounded bg-muted animate-pulse" />
                  <div className="h-3 w-16 rounded bg-muted animate-pulse" />
                </div>
              </div>
            ))}
          </div>
        ) : events.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8 text-center">
            <Activity className="h-8 w-8 text-muted-foreground/50 mb-2" />
            <p className="text-sm text-muted-foreground">No events yet</p>
            <p className="text-xs text-muted-foreground">Create your first event to get started</p>
          </div>
        ) : (
          <div className="space-y-1">
            {events.map((event, index) => (
              <Link
                key={event.id}
                href={`/dashboard/events?entity=${event.entity_id}`}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-2 py-2 transition-colors hover:bg-muted",
                  index === 0 && "bg-muted/50"
                )}
              >
                <span
                  className={cn(
                    "h-2 w-2 shrink-0 rounded-full",
                    getEventTypeColor(event.event_type),
                    index === 0 && "animate-pulse"
                  )}
                />
                <div className="flex-1 min-w-0">
                  <p className="truncate text-sm font-medium">{event.event_type}</p>
                  <p className="truncate text-xs text-muted-foreground">{event.entity_id}</p>
                </div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground">
                  <Clock className="h-3 w-3" />
                  {formatTimestamp(event.timestamp)}
                </div>
              </Link>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
