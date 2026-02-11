"use client";

import { useEffect, useState, useRef } from "react";
import { Card, CardContent, CardHeader, CardTitle, Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Radio, Pause, Play, Trash2 } from "lucide-react";
import type { Event } from "@/lib/api/client";

interface LiveEventFeedProps {
  onEventClick?: (event: Event) => void;
}

export function LiveEventFeed({ onEventClick }: LiveEventFeedProps) {
  const [events, setEvents] = useState<Event[]>([]);
  const [isPaused, setIsPaused] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const eventCounterRef = useRef(0);

  // Simulate live events (in production, this would use WebSocket)
  useEffect(() => {
    if (isPaused) return;

    const eventTypes = [
      "user.signed_up",
      "user.logged_in",
      "user.profile_updated",
      "order.placed",
      "order.confirmed",
      "order.shipped",
      "payment.initiated",
      "payment.completed",
      "inventory.updated",
      "inventory.low_stock",
    ];

    const entities = [
      "user-123",
      "user-456",
      "user-789",
      "order-001",
      "order-002",
      "order-003",
      "payment-abc",
      "payment-def",
      "inventory-xyz",
    ];

    const interval = setInterval(() => {
      const newEvent: Event = {
        id: `evt_${Date.now()}_${Math.random().toString(36).slice(2)}`,
        entity_id: entities[Math.floor(Math.random() * entities.length)]!,
        event_type: eventTypes[Math.floor(Math.random() * eventTypes.length)]!,
        payload: {
          source: "live_demo",
          random_value: Math.floor(Math.random() * 1000),
        },
        timestamp: new Date().toISOString(),
        version: ++eventCounterRef.current,
      };

      setEvents((prev) => [newEvent, ...prev].slice(0, 50));
    }, Math.random() * 1500 + 500);

    return () => clearInterval(interval);
  }, [isPaused]);

  const clearEvents = () => {
    setEvents([]);
  };

  const getEventTypeColor = (eventType: string) => {
    if (eventType.includes("user")) return "bg-blue-500";
    if (eventType.includes("order")) return "bg-green-500";
    if (eventType.includes("payment")) return "bg-purple-500";
    if (eventType.includes("inventory")) return "bg-orange-500";
    return "bg-gray-500";
  };

  const formatTime = (timestamp: string) => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  return (
    <Card className="flex h-full flex-col">
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <div className="flex items-center gap-2">
          <div className="relative">
            <Radio className="h-4 w-4 text-red-500" />
            {!isPaused && (
              <span className="absolute -right-0.5 -top-0.5 h-2 w-2 animate-ping rounded-full bg-red-500" />
            )}
          </div>
          <CardTitle className="text-base font-medium">Live Feed</CardTitle>
          <span className="text-xs text-muted-foreground">
            ({events.length} events)
          </span>
        </div>
        <div className="flex items-center gap-1">
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
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={clearEvents}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        <div
          ref={containerRef}
          className="h-[400px] overflow-y-auto px-4 pb-4"
        >
          {events.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {isPaused ? "Feed paused" : "Waiting for events..."}
            </div>
          ) : (
            <div className="space-y-1">
              {events.map((event, index) => (
                <button
                  key={event.id}
                  onClick={() => onEventClick?.(event)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left transition-all hover:bg-muted",
                    index === 0 && !isPaused && "animate-in slide-in-from-top-2 bg-muted/50"
                  )}
                >
                  <span
                    className={cn(
                      "h-2 w-2 shrink-0 rounded-full",
                      getEventTypeColor(event.event_type),
                      index === 0 && !isPaused && "animate-pulse"
                    )}
                  />
                  <span className="min-w-0 flex-1 truncate text-xs font-medium">
                    {event.event_type}
                  </span>
                  <span className="shrink-0 truncate text-xs text-muted-foreground">
                    {event.entity_id.slice(0, 10)}
                  </span>
                  <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                    {formatTime(event.timestamp)}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
