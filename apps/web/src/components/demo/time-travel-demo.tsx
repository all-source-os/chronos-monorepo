"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { Event } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  Calendar,
  ChevronLeft,
  ChevronRight,
  CircleDot,
  Clock,
  Database,
  FastForward,
  History,
  Pause,
  Play,
  Rewind,
  RotateCcw,
  TrendingUp,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

interface TimelinePoint {
  timestamp: string;
  label: string;
  events: Event[];
  eventCount: number;
}

export function TimeTravelDemo() {
  const [events, setEvents] = useState<Event[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedTimestamp, setSelectedTimestamp] = useState<string | null>(null);
  const [entityId, setEntityId] = useState("order-1");
  const [timelinePoints, setTimelinePoints] = useState<TimelinePoint[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [entityState, setEntityState] = useState<Record<string, unknown>>({});

  // Generate timeline points
  const generateTimeline = (events: Event[]): TimelinePoint[] => {
    if (events.length === 0) return [];

    // Group events by hour
    const grouped = events.reduce(
      (acc, event) => {
        const hour = new Date(event.timestamp).toISOString().slice(0, 13);
        if (!acc[hour]) {
          acc[hour] = [];
        }
        acc[hour].push(event);
        return acc;
      },
      {} as Record<string, Event[]>
    );

    return Object.entries(grouped)
      .map(([timestamp, events]) => ({
        timestamp: `${timestamp}:00:00Z`,
        label: new Date(`${timestamp}:00:00Z`).toLocaleString(undefined, {
          month: "short",
          day: "numeric",
          hour: "2-digit",
          minute: "2-digit",
        }),
        events,
        eventCount: events.length,
      }))
      .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
  };

  // Reconstruct entity state at a given point in time
  const reconstructState = (events: Event[]): Record<string, unknown> => {
    const state: Record<string, unknown> = {};
    let eventCount = 0;
    let totalAmount = 0;

    events.forEach((event) => {
      eventCount++;
      if (event.payload.amount && typeof event.payload.amount === "number") {
        totalAmount += event.payload.amount;
      }
      if (event.payload.status) {
        state.status = event.payload.status;
      }
      if (event.payload.customer) {
        state.customer = event.payload.customer;
      }
    });

    return {
      ...state,
      event_count: eventCount,
      total_amount: totalAmount,
      last_event: events[events.length - 1]?.event_type || null,
      last_timestamp: events[events.length - 1]?.timestamp || null,
    };
  };

  const fetchTimelineData = async () => {
    setIsLoading(true);
    try {
      // Get last 24 hours of events for the entity
      const now = new Date();
      const yesterday = new Date(now.getTime() - 24 * 60 * 60 * 1000);

      const data = await eventStoreClient.queryEvents({
        entity_id: entityId,
        from_timestamp: yesterday.toISOString(),
        to_timestamp: now.toISOString(),
        limit: 100,
      });

      setEvents(data);
      const timeline = generateTimeline(data);
      setTimelinePoints(timeline);

      if (timeline.length > 0) {
        const lastIndex = timeline.length - 1;
        const lastTimelinePoint = timeline[lastIndex];
        if (lastTimelinePoint) {
          setCurrentIndex(lastIndex);
          setSelectedTimestamp(lastTimelinePoint.timestamp);

          // Reconstruct state at latest point
          const allEventsUpToNow = data.filter(
            (e) => new Date(e.timestamp) <= new Date(lastTimelinePoint.timestamp)
          );
          setEntityState(reconstructState(allEventsUpToNow));
        }
      }
    } catch (error) {
      console.error("Failed to fetch timeline data:", error);
      // Use mock data for demo
      const eventTypes = ["OrderPlaced", "PaymentProcessed", "OrderShipped", "OrderDelivered"];
      const statuses = ["pending", "processing", "shipped", "delivered"];

      const mockEvents: Event[] = Array.from({ length: 10 }, (_, i) => ({
        id: `event-${i}`,
        event_type: eventTypes[i % 4] as string,
        entity_id: entityId,
        tenant_id: "default",
        payload: {
          amount: (i + 1) * 100,
          status: statuses[i % 4] as string,
          customer: `customer-${Math.floor(i / 2)}`,
        },
        timestamp: new Date(Date.now() - (10 - i) * 60 * 60 * 1000).toISOString(),
        version: i + 1,
      }));

      setEvents(mockEvents);
      const timeline = generateTimeline(mockEvents);
      setTimelinePoints(timeline);

      if (timeline.length > 0) {
        const lastIndex = timeline.length - 1;
        const lastTimelinePoint = timeline[lastIndex];
        if (lastTimelinePoint) {
          setCurrentIndex(lastIndex);
          setSelectedTimestamp(lastTimelinePoint.timestamp);

          const allEventsUpToNow = mockEvents.filter(
            (e) => new Date(e.timestamp) <= new Date(lastTimelinePoint.timestamp)
          );
          setEntityState(reconstructState(allEventsUpToNow));
        }
      }
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchTimelineData();
  }, [entityId]);

  // Auto-play through timeline
  useEffect(() => {
    if (!isPlaying || currentIndex >= timelinePoints.length - 1) {
      if (isPlaying && currentIndex >= timelinePoints.length - 1) {
        setIsPlaying(false);
      }
      return;
    }

    const timer = setTimeout(() => {
      handleNext();
    }, 1500);

    return () => clearTimeout(timer);
  }, [isPlaying, currentIndex, timelinePoints.length]);

  const handlePrevious = () => {
    if (currentIndex > 0) {
      const newIndex = currentIndex - 1;
      const timelinePoint = timelinePoints[newIndex];
      if (timelinePoint) {
        setCurrentIndex(newIndex);
        setSelectedTimestamp(timelinePoint.timestamp);

        // Reconstruct state at this point
        const eventsUpToPoint = events.filter(
          (e) => new Date(e.timestamp) <= new Date(timelinePoint.timestamp)
        );
        setEntityState(reconstructState(eventsUpToPoint));
      }
    }
  };

  const handleNext = () => {
    if (currentIndex < timelinePoints.length - 1) {
      const newIndex = currentIndex + 1;
      const timelinePoint = timelinePoints[newIndex];
      if (timelinePoint) {
        setCurrentIndex(newIndex);
        setSelectedTimestamp(timelinePoint.timestamp);

        // Reconstruct state at this point
        const eventsUpToPoint = events.filter(
          (e) => new Date(e.timestamp) <= new Date(timelinePoint.timestamp)
        );
        setEntityState(reconstructState(eventsUpToPoint));
      }
    }
  };

  const handleReset = () => {
    const firstPoint = timelinePoints[0];
    if (firstPoint) {
      setCurrentIndex(0);
      setSelectedTimestamp(firstPoint.timestamp);

      const eventsUpToPoint = events.filter(
        (e) => new Date(e.timestamp) <= new Date(firstPoint.timestamp)
      );
      setEntityState(reconstructState(eventsUpToPoint));
    }
  };

  const handleJumpToLatest = () => {
    if (timelinePoints.length > 0) {
      const newIndex = timelinePoints.length - 1;
      const timelinePoint = timelinePoints[newIndex];
      if (timelinePoint) {
        setCurrentIndex(newIndex);
        setSelectedTimestamp(timelinePoint.timestamp);

        const eventsUpToPoint = events.filter(
          (e) => new Date(e.timestamp) <= new Date(timelinePoint.timestamp)
        );
        setEntityState(reconstructState(eventsUpToPoint));
      }
    }
  };

  const currentEvents =
    timelinePoints[currentIndex]?.events.filter(
      (e) => new Date(e.timestamp) <= new Date(selectedTimestamp || Date.now())
    ) || [];

  const progress =
    timelinePoints.length > 0 ? ((currentIndex + 1) / timelinePoints.length) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Temporal Queries</h2>
          <p className="text-sm text-slate-400 mt-1">Query historical state at any point in time</p>
        </div>
        <div className="flex items-center gap-3">
          <input
            type="text"
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            placeholder="Entity ID"
            className="px-3 py-1 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
          />
          <Button
            onClick={fetchTimelineData}
            disabled={isLoading}
            size="sm"
            variant="outline"
            className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
          >
            <History className="h-4 w-4 mr-2" />
            Load Timeline
          </Button>
        </div>
      </div>

      {/* Entity State Display */}
      {Object.keys(entityState).length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-6 bg-gradient-to-br from-indigo-950/30 to-purple-950/30 rounded-xl border border-indigo-500/20"
        >
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="p-3 bg-indigo-500/20 rounded-lg">
                <Database className="h-8 w-8 text-indigo-400" />
              </div>
              <div>
                <h3 className="text-xl font-bold text-white">Entity State</h3>
                <p className="text-sm text-slate-400">
                  {selectedTimestamp ? new Date(selectedTimestamp).toLocaleString() : "Current"}
                </p>
              </div>
            </div>
            <div className="text-right">
              <p className="text-xs text-slate-400">Entity ID</p>
              <p className="text-sm font-semibold text-white">{entityId}</p>
            </div>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {Object.entries(entityState).map(([key, value]) => (
              <div key={key} className="p-4 bg-slate-900/50 rounded-lg">
                <p className="text-xs text-slate-400 mb-1">
                  {key
                    .split("_")
                    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
                    .join(" ")}
                </p>
                <p className="text-lg font-bold text-white">
                  {typeof value === "number" ? value.toLocaleString() : String(value)}
                </p>
              </div>
            ))}
          </div>
        </motion.div>
      )}

      {/* Timeline Controls */}
      <div className="p-6 bg-slate-800/70 rounded-lg border border-slate-600 space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-white flex items-center gap-2">
            <Clock className="h-5 w-5 text-indigo-400" />
            Time Travel Controls
          </h3>
          <div className="flex items-center gap-2">
            <Button
              onClick={handleReset}
              disabled={isLoading || timelinePoints.length === 0}
              size="sm"
              variant="outline"
              className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
            >
              <RotateCcw className="h-4 w-4" />
            </Button>
            <Button
              onClick={handlePrevious}
              disabled={isLoading || currentIndex === 0}
              size="sm"
              variant="outline"
              className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <Button
              onClick={() => setIsPlaying(!isPlaying)}
              disabled={isLoading || timelinePoints.length === 0}
              size="sm"
              className="bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 text-white"
            >
              {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
            </Button>
            <Button
              onClick={handleNext}
              disabled={isLoading || currentIndex >= timelinePoints.length - 1}
              size="sm"
              variant="outline"
              className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
            <Button
              onClick={handleJumpToLatest}
              disabled={isLoading || currentIndex >= timelinePoints.length - 1}
              size="sm"
              variant="outline"
              className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
            >
              <FastForward className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Progress Bar */}
        <div className="space-y-2">
          <div className="flex items-center justify-between text-xs text-slate-400">
            <span>
              Point {currentIndex + 1} of {timelinePoints.length}
            </span>
            <span>{Math.round(progress)}% of timeline</span>
          </div>
          <div className="w-full bg-slate-700 rounded-full h-3 overflow-hidden">
            <motion.div
              animate={{ width: `${progress}%` }}
              transition={{ duration: 0.3 }}
              className="h-full bg-gradient-to-r from-indigo-500 to-purple-500 rounded-full"
            />
          </div>
        </div>

        {/* Timeline Visualization */}
        <div className="relative pt-4">
          <div className="flex items-center justify-between">
            {timelinePoints.length > 0 ? (
              timelinePoints.map((point, index) => (
                <motion.button
                  key={point.timestamp}
                  onClick={() => {
                    setCurrentIndex(index);
                    setSelectedTimestamp(point.timestamp);
                    const eventsUpToPoint = events.filter(
                      (e) => new Date(e.timestamp) <= new Date(point.timestamp)
                    );
                    setEntityState(reconstructState(eventsUpToPoint));
                  }}
                  initial={{ opacity: 0, scale: 0 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ delay: index * 0.05 }}
                  className="relative flex flex-col items-center group"
                >
                  <motion.div
                    whileHover={{ scale: 1.2 }}
                    className={`w-4 h-4 rounded-full border-2 ${
                      index === currentIndex
                        ? "bg-indigo-500 border-indigo-400 shadow-lg shadow-indigo-500/50"
                        : index < currentIndex
                          ? "bg-purple-600 border-purple-500"
                          : "bg-slate-700 border-slate-600"
                    }`}
                  >
                    {index === currentIndex && (
                      <motion.div
                        animate={{ scale: [1, 1.5, 1] }}
                        transition={{ duration: 1.5, repeat: Number.POSITIVE_INFINITY }}
                        className="absolute inset-0 rounded-full bg-indigo-400 opacity-50"
                      />
                    )}
                  </motion.div>
                  <div className="absolute top-6 text-xs text-slate-400 whitespace-nowrap opacity-0 group-hover:opacity-100 transition-opacity">
                    {point.label}
                  </div>
                  <div className="absolute top-10 text-xs text-indigo-400 font-semibold opacity-0 group-hover:opacity-100 transition-opacity">
                    {point.eventCount} events
                  </div>
                </motion.button>
              ))
            ) : (
              <div className="text-center text-slate-400 py-4 w-full">
                {isLoading ? "Loading timeline..." : "No timeline data available"}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Events at Selected Time */}
      {currentEvents.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-3"
        >
          <div className="flex items-center gap-2 p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <Zap className="h-5 w-5 text-yellow-400" />
            <h3 className="text-lg font-semibold text-white">
              Events at {timelinePoints[currentIndex]?.label}
            </h3>
            <span className="text-sm text-slate-400">({currentEvents.length} events)</span>
          </div>

          <div className="grid grid-cols-1 gap-2 max-h-64 overflow-y-auto">
            {currentEvents.map((event, index) => (
              <motion.div
                key={event.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.03 }}
                className="p-4 bg-slate-800/70 rounded-lg border border-slate-700"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <h4 className="font-semibold text-white text-sm">{event.event_type}</h4>
                      <span className="text-xs text-slate-400">v{event.version}</span>
                    </div>
                    <p className="text-xs text-slate-400">
                      {new Date(event.timestamp).toLocaleString()}
                    </p>
                    <pre className="text-xs text-slate-300 mt-2 bg-slate-900/50 p-2 rounded overflow-x-auto">
                      {JSON.stringify(event.payload, null, 2)}
                    </pre>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </motion.div>
      )}

      {/* Info Box */}
      <div className="p-4 bg-indigo-950/30 rounded-lg border border-indigo-500/20">
        <div className="flex items-start gap-3">
          <CircleDot className="h-5 w-5 text-indigo-400 mt-0.5" />
          <div>
            <h4 className="text-sm font-semibold text-white mb-1">How Time Travel Works</h4>
            <p className="text-xs text-slate-400">
              Navigate through your entity's history using the timeline controls. The entity state
              is reconstructed by replaying events up to the selected point in time, allowing you to
              see exactly how the entity looked at any moment in its history.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
