"use client";

import { Button, Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Eye, EyeOff, Pause, Play, Radio, Trash2, Wifi, WifiOff } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { usePhoenixChannel } from "@/hooks/use-phoenix-channel";
import type { Event } from "@/lib/api/client";
import { isPlatformNoise } from "@/lib/event-namespaces";

interface LiveEventFeedProps {
  onEventClick?: (event: Event) => void;
}

export function LiveEventFeed({ onEventClick }: LiveEventFeedProps) {
  const [events, setEvents] = useState<Event[]>([]);
  const [isPaused, setIsPaused] = useState(false);
  // The `events:all` channel streams platform-noise too (e.g. service.heartbeat
  // from the system tenant). Drop it at ingest by default so the feed shows
  // domain activity — and so a flood of heartbeats can't push real events out
  // of the 50-item window. Toggle on to inspect system activity.
  const [showSystem, setShowSystem] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Handle incoming Phoenix Channel events
  const handleChannelEvent = useCallback(
    (data: unknown) => {
      if (isPaused) return;
      const event = data as Event;
      if (!event?.id) return;
      if (!showSystem && isPlatformNoise(event.event_type)) return;
      setEvents((prev) => [event, ...prev].slice(0, 50));
    },
    [isPaused, showSystem]
  );

  const { isConnected, status, connect } = usePhoenixChannel("events:all", {
    onEvent: handleChannelEvent,
  });

  const disconnectedMessage = (() => {
    switch (status) {
      case "unconfigured":
        return "Live stream not configured for this environment";
      case "unauthenticated":
        return "Sign in to stream live events";
      case "connecting":
        return "Connecting to live stream…";
      default:
        return "No events — WebSocket disconnected";
    }
  })();

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
          {isConnected ? (
            <span title="WebSocket connected">
              <Wifi className="h-3 w-3 text-green-500" />
            </span>
          ) : (
            <span title="Connecting...">
              <WifiOff className="h-3 w-3 text-muted-foreground" />
            </span>
          )}
          <span className="text-xs text-muted-foreground">({events.length})</span>
        </div>
        <div className="flex items-center gap-1">
          {!isConnected && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 text-xs"
              onClick={() => connect()}
              title="Try to connect to live stream"
            >
              <Wifi className="mr-1 h-3 w-3" />
              Connect
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setShowSystem((s) => !s)}
            title={showSystem ? "Hide system events (heartbeats, audit)" : "Show system events"}
          >
            {showSystem ? <Eye className="h-3.5 w-3.5" /> : <EyeOff className="h-3.5 w-3.5" />}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsPaused(!isPaused)}
          >
            {isPaused ? <Play className="h-3.5 w-3.5" /> : <Pause className="h-3.5 w-3.5" />}
          </Button>
          <Button variant="ghost" size="icon" className="h-7 w-7" onClick={clearEvents}>
            <Trash2 className="h-3.5 w-3.5" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        <div ref={containerRef} className="h-[400px] overflow-y-auto px-4 pb-4">
          {events.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {isPaused
                ? "Feed paused"
                : isConnected
                  ? "Waiting for events..."
                  : disconnectedMessage}
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
