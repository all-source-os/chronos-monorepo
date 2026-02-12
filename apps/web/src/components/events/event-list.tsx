"use client";

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Check, ChevronDown, ChevronRight, Clock, Copy } from "lucide-react";
import { useState } from "react";
import type { Event } from "@/lib/api/client";

interface EventListProps {
  events: Event[];
  onEventClick?: (event: Event) => void;
  selectedEventId?: string;
  isLoading?: boolean;
}

function EventRow({
  event,
  isSelected,
  onClick,
}: {
  event: Event;
  isSelected: boolean;
  onClick?: () => void;
}) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [copied, setCopied] = useState(false);

  const getEventTypeColor = (eventType: string) => {
    if (eventType.includes("user")) return "bg-blue-500/10 text-blue-600 dark:text-blue-400";
    if (eventType.includes("order")) return "bg-green-500/10 text-green-600 dark:text-green-400";
    if (eventType.includes("payment"))
      return "bg-purple-500/10 text-purple-600 dark:text-purple-400";
    if (eventType.includes("inventory"))
      return "bg-orange-500/10 text-orange-600 dark:text-orange-400";
    if (eventType.includes("error")) return "bg-red-500/10 text-red-600 dark:text-red-400";
    return "bg-muted text-muted-foreground";
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return {
      date: date.toLocaleDateString(),
      time: date.toLocaleTimeString(),
    };
  };

  const copyToClipboard = async (text: string) => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const { date, time } = formatTimestamp(event.timestamp);

  return (
    <div className={cn("border-b border-border transition-colors", isSelected && "bg-primary/5")}>
      {/* Main row */}
      <div
        className="flex cursor-pointer items-center gap-3 px-4 py-3 hover:bg-muted/50"
        onClick={() => {
          onClick?.();
        }}
      >
        {/* Expand toggle */}
        <button
          onClick={(e) => {
            e.stopPropagation();
            setIsExpanded(!isExpanded);
          }}
          className="shrink-0 p-1 hover:bg-muted rounded"
        >
          {isExpanded ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
        </button>

        {/* Event type badge */}
        <span
          className={cn(
            "shrink-0 rounded-full px-2.5 py-0.5 text-xs font-medium",
            getEventTypeColor(event.event_type)
          )}
        >
          {event.event_type}
        </span>

        {/* Entity ID */}
        <span className="min-w-0 flex-1 truncate text-sm font-medium">{event.entity_id}</span>

        {/* Timestamp */}
        <div className="hidden shrink-0 items-center gap-1.5 text-xs text-muted-foreground sm:flex">
          <Clock className="h-3 w-3" />
          <span>{date}</span>
          <span className="text-muted-foreground/60">{time}</span>
        </div>

        {/* Version */}
        <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
          v{event.version}
        </span>
      </div>

      {/* Expanded content */}
      {isExpanded && (
        <div className="border-t border-border bg-muted/30 px-4 py-3">
          <div className="flex items-start justify-between gap-4">
            <div className="min-w-0 flex-1">
              <p className="mb-2 text-xs font-medium text-muted-foreground">Event Payload</p>
              <pre className="overflow-x-auto rounded-lg bg-background p-3 text-xs">
                <code className="text-foreground">{JSON.stringify(event.payload, null, 2)}</code>
              </pre>
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 shrink-0"
              onClick={(e) => {
                e.stopPropagation();
                copyToClipboard(JSON.stringify(event.payload, null, 2));
              }}
            >
              {copied ? (
                <Check className="h-3.5 w-3.5 text-green-500" />
              ) : (
                <Copy className="h-3.5 w-3.5" />
              )}
            </Button>
          </div>

          <div className="mt-3 grid grid-cols-2 gap-4 text-xs sm:grid-cols-4">
            <div>
              <p className="text-muted-foreground">Event ID</p>
              <p className="font-mono">{event.id.slice(0, 8)}...</p>
            </div>
            <div>
              <p className="text-muted-foreground">Entity ID</p>
              <p className="font-mono">{event.entity_id}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Version</p>
              <p className="font-mono">{event.version}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Timestamp</p>
              <p className="font-mono">{new Date(event.timestamp).toISOString()}</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function EventList({ events, onEventClick, selectedEventId, isLoading }: EventListProps) {
  if (isLoading) {
    return (
      <div className="divide-y divide-border rounded-lg border border-border">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="flex items-center gap-3 px-4 py-3">
            <div className="h-4 w-4 rounded bg-muted animate-pulse" />
            <div className="h-5 w-20 rounded-full bg-muted animate-pulse" />
            <div className="h-4 flex-1 rounded bg-muted animate-pulse" />
            <div className="h-4 w-24 rounded bg-muted animate-pulse" />
          </div>
        ))}
      </div>
    );
  }

  if (events.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-12 text-center">
        <p className="text-sm text-muted-foreground">No events found</p>
        <p className="text-xs text-muted-foreground">
          Try adjusting your filters or create a new event
        </p>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border">
      {events.map((event) => (
        <EventRow
          key={event.id}
          event={event}
          isSelected={event.id === selectedEventId}
          onClick={() => onEventClick?.(event)}
        />
      ))}
    </div>
  );
}
