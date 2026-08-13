"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { History, Lock, Pause, Play, Radio, Unlock } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { usePhoenixChannel } from "@/hooks/use-phoenix-channel";

/**
 * Event shape matching Core's WebSocket event format.
 */
interface StreamEvent {
  id: string;
  entity_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  timestamp: string;
  version: number;
}

type EventCategory = "ingested" | "vector-indexed" | "keyword-hit";

function categorizeEvent(event: StreamEvent): EventCategory {
  if (event.payload._embedding || event.event_type.includes("vector")) {
    return "vector-indexed";
  }
  if (event.event_type.includes("search") || event.event_type.includes("keyword")) {
    return "keyword-hit";
  }
  return "ingested";
}

const CATEGORY_COLORS: Record<EventCategory, string> = {
  ingested: "bg-green-500",
  "vector-indexed": "bg-blue-500",
  "keyword-hit": "bg-yellow-500",
};

const CATEGORY_ROW_COLORS: Record<EventCategory, string> = {
  ingested: "hover:bg-green-500/5",
  "vector-indexed": "hover:bg-blue-500/5",
  "keyword-hit": "hover:bg-yellow-500/5",
};

const CATEGORY_LABELS: Record<EventCategory, string> = {
  ingested: "Ingested",
  "vector-indexed": "Vector-indexed",
  "keyword-hit": "Keyword hit",
};

const MAX_EVENTS = 200;

function formatTime(ts: string): string {
  const d = new Date(ts);
  return d.toLocaleTimeString("en-US", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

interface StatsState {
  currentRate: number; // events/sec
  peakRate: number;
}

export function LiveEventStreamPanel({ initialEvents = [] }: { initialEvents?: StreamEvent[] }) {
  const [events, setEvents] = useState<StreamEvent[]>(initialEvents);
  const [isPaused, setIsPaused] = useState(false);
  const [scrollLock, setScrollLock] = useState(false);
  const [hoveredEvent, setHoveredEvent] = useState<StreamEvent | null>(null);
  const [tooltipPos, setTooltipPos] = useState({ x: 0, y: 0 });
  const [stats, setStats] = useState<StatsState>({
    currentRate: 0,
    peakRate: 0,
  });
  const [isReplaying, setIsReplaying] = useState(false);
  const [replayedEventIds, setReplayedEventIds] = useState<Set<string>>(new Set());

  const containerRef = useRef<HTMLDivElement>(null);
  const rateWindowRef = useRef<number[]>([]); // timestamps of events in last second
  const replayTimeoutRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  useEffect(() => {
    setEvents(initialEvents);
  }, [initialEvents]);

  // Update throughput stats
  const recordEvent = useCallback((timestamp: number) => {
    rateWindowRef.current.push(timestamp);
    const now = Date.now();
    // Keep only events from last 2 seconds
    rateWindowRef.current = rateWindowRef.current.filter((t) => now - t < 2000);
    const rate = Math.round(rateWindowRef.current.length / 2);
    setStats((prev) => ({
      currentRate: rate,
      peakRate: Math.max(prev.peakRate, rate),
    }));
  }, []);

  // Clean up replay timeouts on unmount
  useEffect(() => {
    return () => {
      replayTimeoutRef.current.forEach(clearTimeout);
    };
  }, []);

  // Replay loaded events locally. This remains useful when WebSockets are not
  // configured and avoids pretending an old event is part of the last 10s.
  const handleReplay = useCallback(async () => {
    setIsReplaying(true);
    setReplayedEventIds(new Set());
    const replayEvents = events.filter((event) => !event.id.startsWith("replay_")).reverse();

    if (replayEvents.length === 0) {
      setIsReplaying(false);
      return;
    }

    // Pause live feed during replay
    setIsPaused(true);

    // Clear any previous replay timeouts
    replayTimeoutRef.current.forEach(clearTimeout);
    replayTimeoutRef.current = [];

    // Calculate replay speed: spread events over ~3 seconds (accelerated)
    const intervalMs = Math.max(30, 3000 / replayEvents.length);

    replayEvents.forEach((event, index) => {
      const replayId = `replay_${event.id}`;
      const replayEvent: StreamEvent = {
        ...event,
        id: replayId,
      };

      const timeout = setTimeout(() => {
        setReplayedEventIds((prev) => new Set(prev).add(replayId));
        setEvents((prev) => [replayEvent, ...prev].slice(0, MAX_EVENTS));
        recordEvent(Date.now());

        // After last event, end replay
        if (index === replayEvents.length - 1) {
          setTimeout(() => {
            setIsReplaying(false);
            setIsPaused(false);
          }, 500);
        }
      }, index * intervalMs);

      replayTimeoutRef.current.push(timeout);
    });
  }, [events, recordEvent]);

  // Handle incoming Phoenix Channel events
  const handleChannelEvent = useCallback(
    (data: unknown) => {
      if (isPaused) return;
      const event = data as StreamEvent;
      if (event?.id) {
        setEvents((prev) => [event, ...prev].slice(0, MAX_EVENTS));
        recordEvent(Date.now());
      }
    },
    [isPaused, recordEvent]
  );

  const { isConnected, status } = usePhoenixChannel("events:all", {
    onEvent: handleChannelEvent,
  });

  // Auto-scroll when not locked
  useEffect(() => {
    if (!scrollLock && events.length > 0 && containerRef.current) {
      containerRef.current.scrollTop = 0;
    }
  }, [events, scrollLock]);

  // Periodically decay stats when paused / no events
  useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now();
      rateWindowRef.current = rateWindowRef.current.filter((t) => now - t < 2000);
      const rate = Math.round(rateWindowRef.current.length / 2);
      setStats((prev) => ({
        ...prev,
        currentRate: rate,
      }));
    }, 500);
    return () => clearInterval(interval);
  }, []);

  const handleMouseEnter = useCallback(
    (event: StreamEvent, e: React.SyntheticEvent<HTMLElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      setTooltipPos({ x: rect.right + 8, y: rect.top });
      setHoveredEvent(event);
    },
    []
  );

  const handleMouseLeave = useCallback(() => {
    setHoveredEvent(null);
  }, []);

  return (
    <Card
      className="flex h-full flex-col"
      data-testid="event-stream-panel"
      role="region"
      aria-label="Live event stream"
    >
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <div className="relative">
            <Radio className="h-4 w-4 text-red-500" />
            {isConnected && !isPaused && (
              <span className="absolute -right-0.5 -top-0.5 h-2 w-2 animate-ping rounded-full bg-red-500" />
            )}
          </div>
          <h3 className="text-sm font-semibold">Event stream</h3>
          <span
            className={cn(
              "rounded-full border px-2 py-0.5 text-[10px] font-medium",
              isConnected
                ? "border-emerald-500/30 bg-emerald-500/10 text-emerald-500"
                : "border-border bg-muted/40 text-muted-foreground"
            )}
          >
            {isConnected ? "Live" : status === "connecting" ? "Connecting" : "Stored"}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs"
            onClick={handleReplay}
            disabled={isReplaying}
            aria-label="Replay loaded events"
            data-testid="replay-button"
          >
            <History className={cn("mr-1 h-3 w-3", isReplaying && "animate-spin")} />
            {isReplaying ? "Replaying…" : "Replay loaded"}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setScrollLock(!scrollLock)}
            aria-label={scrollLock ? "Unlock scroll" : "Lock scroll"}
          >
            {scrollLock ? <Lock className="h-3.5 w-3.5" /> : <Unlock className="h-3.5 w-3.5" />}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsPaused(!isPaused)}
            aria-label={isPaused ? "Resume" : "Pause"}
          >
            {isPaused ? <Play className="h-3.5 w-3.5" /> : <Pause className="h-3.5 w-3.5" />}
          </Button>
        </div>
      </div>

      {/* Stats overlay bar */}
      <div
        className="flex items-center gap-4 border-b border-border bg-muted/30 px-4 py-1.5 text-xs font-mono"
        data-testid="stats-bar"
      >
        <span>
          Loaded: <span className="font-semibold text-foreground">{events.length}</span>
        </span>
        <span>
          Live:{" "}
          <span className="font-semibold text-foreground">
            {stats.currentRate >= 1000
              ? `${(stats.currentRate / 1000).toFixed(1)}k`
              : stats.currentRate}
            /sec
          </span>
        </span>
        <span>
          Peak:{" "}
          <span className="font-semibold text-foreground">
            {stats.peakRate >= 1000 ? `${(stats.peakRate / 1000).toFixed(1)}k` : stats.peakRate}
            /sec
          </span>
        </span>
      </div>

      {/* Event list */}
      <CardContent className="flex-1 overflow-hidden p-0">
        <div
          ref={containerRef}
          className="h-[480px] overflow-y-auto"
          data-testid="event-list"
          role="log"
          aria-label="Event feed"
        >
          {events.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              {isPaused ? "Feed paused" : "No events loaded"}
            </div>
          ) : (
            <div className="divide-y divide-border/50">
              {events.map((event, index) => {
                const category = categorizeEvent(event);
                return (
                  <button
                    type="button"
                    key={event.id}
                    className={cn(
                      "flex w-full items-center gap-2 px-4 py-1.5 text-left text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
                      CATEGORY_ROW_COLORS[category],
                      replayedEventIds.has(event.id)
                        ? "ring-1 ring-purple-500/50 bg-purple-500/10"
                        : index === 0 && !isPaused && "animate-in slide-in-from-top-1 duration-200"
                    )}
                    data-replayed={replayedEventIds.has(event.id) || undefined}
                    onMouseEnter={(e) => handleMouseEnter(event, e)}
                    onMouseLeave={handleMouseLeave}
                    onFocus={(e) => handleMouseEnter(event, e)}
                    onBlur={handleMouseLeave}
                    data-testid="event-row"
                  >
                    {/* Category dot */}
                    <span
                      className={cn(
                        "h-2 w-2 shrink-0 rounded-full",
                        CATEGORY_COLORS[category],
                        index === 0 && !isPaused && "animate-pulse"
                      )}
                      title={CATEGORY_LABELS[category]}
                    />

                    {/* Event type */}
                    <span className="min-w-0 flex-1 truncate font-medium">{event.event_type}</span>

                    {/* Entity */}
                    <span className="shrink-0 text-muted-foreground max-w-[80px] truncate">
                      {event.entity_id}
                    </span>

                    {/* Timestamp */}
                    <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                      {formatTime(event.timestamp)}
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </CardContent>

      {/* Legend */}
      <div className="flex items-center gap-4 border-t border-border px-4 py-1.5 text-[10px] text-muted-foreground">
        <span className="flex items-center gap-1">
          <span className="h-1.5 w-1.5 rounded-full bg-green-500" /> Ingested
        </span>
        <span className="flex items-center gap-1">
          <span className="h-1.5 w-1.5 rounded-full bg-blue-500" /> Vector
        </span>
        <span className="flex items-center gap-1">
          <span className="h-1.5 w-1.5 rounded-full bg-yellow-500" /> Keyword
        </span>
      </div>

      {/* Hover tooltip — rendered as portal-like fixed element */}
      {hoveredEvent && (
        <div
          className="fixed z-50 max-w-sm rounded-lg border border-border bg-popover p-3 shadow-lg text-xs"
          style={{
            left: Math.min(tooltipPos.x, window.innerWidth - 360),
            top: Math.min(tooltipPos.y, window.innerHeight - 200),
          }}
        >
          <div className="mb-2 flex items-center justify-between gap-4">
            <span className="font-semibold">{hoveredEvent.event_type}</span>
            <span className="font-mono text-muted-foreground">
              {new Date(hoveredEvent.timestamp).toISOString()}
            </span>
          </div>
          <pre className="max-h-[150px] overflow-auto rounded bg-muted p-2 text-[10px]">
            {JSON.stringify(hoveredEvent.payload, null, 2)}
          </pre>
          <div className="mt-1 text-muted-foreground">
            ID: {hoveredEvent.id} | Entity: {hoveredEvent.entity_id}
          </div>
        </div>
      )}
    </Card>
  );
}
