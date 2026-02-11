"use client";

import { useState, useRef, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Clock, ZoomIn, ZoomOut, RotateCcw } from "lucide-react";
import { Button } from "@allsource/ui";
import type { Event } from "@/lib/api/client";

interface EventTimelineProps {
  events: Event[];
  onEventClick?: (event: Event) => void;
  selectedEventId?: string;
}

export function EventTimeline({
  events,
  onEventClick,
  selectedEventId,
}: EventTimelineProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState(0);
  const [hoveredEvent, setHoveredEvent] = useState<Event | null>(null);
  const [tooltipPosition, setTooltipPosition] = useState({ x: 0, y: 0 });

  // Sort events by timestamp
  const sortedEvents = [...events].sort(
    (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
  );

  // Calculate time range
  const minTime =
    sortedEvents.length > 0
      ? new Date(sortedEvents[0]!.timestamp).getTime()
      : Date.now();
  const maxTime =
    sortedEvents.length > 0
      ? new Date(sortedEvents[sortedEvents.length - 1]!.timestamp).getTime()
      : Date.now();
  const timeRange = maxTime - minTime || 1;

  // Get event position as percentage
  const getEventPosition = (event: Event) => {
    const eventTime = new Date(event.timestamp).getTime();
    return ((eventTime - minTime) / timeRange) * 100;
  };

  // Get color based on event type
  const getEventColor = (eventType: string) => {
    if (eventType.includes("user")) return "bg-blue-500";
    if (eventType.includes("order")) return "bg-green-500";
    if (eventType.includes("payment")) return "bg-purple-500";
    if (eventType.includes("inventory")) return "bg-orange-500";
    if (eventType.includes("error")) return "bg-red-500";
    return "bg-primary";
  };

  // Handle zoom
  const handleZoom = (delta: number) => {
    setZoom((prev) => Math.max(1, Math.min(10, prev + delta)));
  };

  const resetView = () => {
    setZoom(1);
    setPan(0);
  };

  // Handle pan
  const handleMouseDown = (e: React.MouseEvent) => {
    if (zoom <= 1) return;
    setIsDragging(true);
    setDragStart(e.clientX - pan);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging) return;
    const newPan = e.clientX - dragStart;
    const maxPan = (containerRef.current?.offsetWidth || 0) * (zoom - 1) * 0.5;
    setPan(Math.max(-maxPan, Math.min(maxPan, newPan)));
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Handle event hover
  const handleEventHover = (event: Event, e: React.MouseEvent) => {
    setHoveredEvent(event);
    setTooltipPosition({ x: e.clientX, y: e.clientY });
  };

  // Format time for display
  const formatTime = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <div className="flex items-center gap-2">
          <Clock className="h-4 w-4 text-muted-foreground" />
          <CardTitle className="text-base font-medium">Event Timeline</CardTitle>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => handleZoom(-0.5)}
            disabled={zoom <= 1}
          >
            <ZoomOut className="h-3.5 w-3.5" />
          </Button>
          <span className="min-w-[40px] text-center text-xs text-muted-foreground">
            {zoom.toFixed(1)}x
          </span>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => handleZoom(0.5)}
            disabled={zoom >= 10}
          >
            <ZoomIn className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={resetView}
          >
            <RotateCcw className="h-3.5 w-3.5" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {events.length === 0 ? (
          <div className="flex h-24 items-center justify-center text-sm text-muted-foreground">
            No events to display
          </div>
        ) : (
          <>
            {/* Timeline container */}
            <div
              ref={containerRef}
              className={cn(
                "relative h-24 overflow-hidden rounded-lg bg-muted/30",
                zoom > 1 && "cursor-grab",
                isDragging && "cursor-grabbing"
              )}
              onMouseDown={handleMouseDown}
              onMouseMove={handleMouseMove}
              onMouseUp={handleMouseUp}
              onMouseLeave={handleMouseUp}
            >
              {/* Timeline content */}
              <div
                className="absolute inset-0 transition-transform duration-100"
                style={{
                  transform: `translateX(${pan}px) scaleX(${zoom})`,
                  transformOrigin: "center",
                }}
              >
                {/* Timeline track */}
                <div className="absolute left-4 right-4 top-1/2 h-0.5 -translate-y-1/2 bg-border" />

                {/* Event dots */}
                {sortedEvents.map((event, index) => {
                  const position = getEventPosition(event);
                  const isSelected = event.id === selectedEventId;

                  return (
                    <button
                      key={event.id}
                      className={cn(
                        "absolute top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full transition-all hover:scale-150",
                        getEventColor(event.event_type),
                        isSelected ? "h-4 w-4 ring-2 ring-primary ring-offset-2" : "h-3 w-3"
                      )}
                      style={{ left: `calc(${position}% * 0.92 + 4%)` }}
                      onClick={() => onEventClick?.(event)}
                      onMouseEnter={(e) => handleEventHover(event, e)}
                      onMouseLeave={() => setHoveredEvent(null)}
                    />
                  );
                })}
              </div>
            </div>

            {/* Time labels */}
            <div className="mt-2 flex justify-between text-xs text-muted-foreground">
              <span>{sortedEvents[0] ? formatTime(sortedEvents[0].timestamp) : ""}</span>
              <span>{sortedEvents[sortedEvents.length - 1] ? formatTime(sortedEvents[sortedEvents.length - 1]!.timestamp) : ""}</span>
            </div>

            {/* Tooltip */}
            {hoveredEvent && (
              <div
                className="fixed z-50 max-w-xs rounded-lg border border-border bg-popover p-3 shadow-lg"
                style={{
                  left: tooltipPosition.x + 10,
                  top: tooltipPosition.y - 80,
                }}
              >
                <p className="font-medium">{hoveredEvent.event_type}</p>
                <p className="text-xs text-muted-foreground">
                  {hoveredEvent.entity_id}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  {formatTime(hoveredEvent.timestamp)}
                </p>
              </div>
            )}

            {/* Legend */}
            <div className="mt-4 flex flex-wrap gap-3">
              {[
                { type: "user", color: "bg-blue-500" },
                { type: "order", color: "bg-green-500" },
                { type: "payment", color: "bg-purple-500" },
                { type: "inventory", color: "bg-orange-500" },
              ].map(({ type, color }) => (
                <div key={type} className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <span className={cn("h-2 w-2 rounded-full", color)} />
                  <span className="capitalize">{type}</span>
                </div>
              ))}
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
