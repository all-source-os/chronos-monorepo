"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import { generateEcommerceEvents, generateIoTEvents } from "@/lib/event-store/demo-data";
import type { CreateEventRequest } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  Activity,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Loader2,
  ShoppingCart,
  Thermometer,
  Trash2,
  TrendingUp,
  XCircle,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

interface EventBatch {
  type: string;
  count: number;
  status: "success" | "error";
  timestamp: number;
  events: CreateEventRequest[];
  duration: number;
}

export function EventIngestionDemo() {
  const [isLoading, setIsLoading] = useState(false);
  const [events, setEvents] = useState<EventBatch[]>([]);
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);
  const [batchSize, setBatchSize] = useState(10);
  const [throughput, setThroughput] = useState(0);

  // Calculate throughput based on recent events
  useEffect(() => {
    if (events.length === 0) {
      setThroughput(0);
      return;
    }

    const now = Date.now();
    const recentEvents = events.filter((e) => now - e.timestamp < 10000); // Last 10 seconds
    const totalEvents = recentEvents.reduce((sum, e) => sum + e.count, 0);
    const throughputRate = totalEvents / 10; // events per second

    setThroughput(throughputRate);
  }, [events]);

  const ingestEvents = async (type: "ecommerce" | "iot") => {
    setIsLoading(true);
    const startTime = Date.now();
    const newEvents =
      type === "ecommerce" ? generateEcommerceEvents(batchSize) : generateIoTEvents(batchSize);

    try {
      await eventStoreClient.createEventBatch(newEvents);
      const duration = Date.now() - startTime;
      setEvents((prev) => [
        ...prev,
        {
          type,
          count: newEvents.length,
          status: "success",
          timestamp: Date.now(),
          events: newEvents,
          duration,
        },
      ]);
    } catch (error) {
      const duration = Date.now() - startTime;
      setEvents((prev) => [
        ...prev,
        {
          type,
          count: newEvents.length,
          status: "error",
          timestamp: Date.now(),
          events: newEvents,
          duration,
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const clearEvents = () => {
    setEvents([]);
    setExpandedIndex(null);
    setThroughput(0);
  };

  // Calculate event type breakdown
  const eventTypeBreakdown = events.reduce(
    (acc, batch) => {
      batch.events.forEach((event) => {
        acc[event.event_type] = (acc[event.event_type] || 0) + 1;
      });
      return acc;
    },
    {} as Record<string, number>
  );

  const totalIngestedEvents = events.reduce((sum, e) => sum + e.count, 0);
  const topEventTypes = Object.entries(eventTypeBreakdown)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 5);

  return (
    <div className="relative space-y-6">
      {/* Batch Size Selector */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <label className="text-sm font-medium text-slate-300">Batch Size:</label>
          <select
            value={batchSize}
            onChange={(e) => setBatchSize(Number(e.target.value))}
            disabled={isLoading}
            className="px-3 py-1 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value={5}>5 events</option>
            <option value={10}>10 events</option>
            <option value={25}>25 events</option>
            <option value={50}>50 events</option>
            <option value={100}>100 events</option>
          </select>
        </div>
        {events.length > 0 && (
          <Button
            onClick={clearEvents}
            disabled={isLoading}
            variant="outline"
            size="sm"
            className="bg-slate-700/50 hover:bg-red-600/20 text-white border-slate-600"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Clear All
          </Button>
        )}
      </div>

      {/* Generation Buttons */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Button
          onClick={() => ingestEvents("ecommerce")}
          disabled={isLoading}
          className="h-32 text-lg font-semibold bg-gradient-to-br from-blue-600 to-blue-700 hover:from-blue-500 hover:to-blue-600 text-white border-blue-500"
        >
          <motion.div
            className="flex flex-col items-center gap-3"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            {isLoading ? (
              <Loader2 className="h-8 w-8 animate-spin text-white" />
            ) : (
              <ShoppingCart className="h-8 w-8 text-white" />
            )}
            <span className="text-white">Generate E-Commerce Events</span>
            <span className="text-sm text-blue-100">Orders, Payments, Shipments</span>
          </motion.div>
        </Button>

        <Button
          onClick={() => ingestEvents("iot")}
          disabled={isLoading}
          className="h-32 text-lg font-semibold bg-gradient-to-br from-purple-600 to-purple-700 hover:from-purple-500 hover:to-purple-600 text-white border-purple-500"
        >
          <motion.div
            className="flex flex-col items-center gap-3"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            {isLoading ? (
              <Loader2 className="h-8 w-8 animate-spin text-white" />
            ) : (
              <Thermometer className="h-8 w-8 text-white" />
            )}
            <span className="text-white">Generate IoT Sensor Data</span>
            <span className="text-sm text-purple-100">Temperature, Humidity, Pressure</span>
          </motion.div>
        </Button>
      </div>

      {/* Event Stream Visualization */}
      <div className="min-h-[200px] p-6 bg-slate-800/70 rounded-lg border border-slate-600">
        <h3 className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
          <Activity className="h-4 w-4 text-blue-400" />
          Event Stream
        </h3>
        <AnimatePresence mode="popLayout">
          {events.length === 0 ? (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="text-center text-slate-500 py-12"
            >
              <Zap className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p>Click a button above to start ingesting events</p>
            </motion.div>
          ) : (
            <div className="space-y-2">
              {events
                .slice(-5)
                .reverse()
                .map((batch, index) => (
                  <motion.div
                    key={`${batch.type}-${batch.timestamp}`}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: 20 }}
                    transition={{ duration: 0.3, delay: index * 0.05 }}
                    className={`rounded-md border ${
                      batch.status === "success"
                        ? "bg-emerald-950/30 border-emerald-700/50"
                        : "bg-red-950/30 border-red-700/50"
                    }`}
                  >
                    <div
                      className="p-4 cursor-pointer"
                      onClick={() => setExpandedIndex(expandedIndex === index ? null : index)}
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          {batch.status === "success" ? (
                            <CheckCircle2 className="h-5 w-5 text-emerald-400" />
                          ) : (
                            <XCircle className="h-5 w-5 text-red-400" />
                          )}
                          <div>
                            <p className="font-medium text-sm text-white">
                              {batch.type === "ecommerce" ? "E-Commerce Events" : "IoT Sensor Data"}
                            </p>
                            <p className="text-xs text-slate-400">
                              {batch.count} events • {batch.duration}ms
                            </p>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-slate-500">
                            {new Date(batch.timestamp).toLocaleTimeString()}
                          </span>
                          {expandedIndex === index ? (
                            <ChevronUp className="h-4 w-4 text-slate-400" />
                          ) : (
                            <ChevronDown className="h-4 w-4 text-slate-400" />
                          )}
                        </div>
                      </div>
                    </div>

                    {/* Expanded Event Details */}
                    <AnimatePresence>
                      {expandedIndex === index && (
                        <motion.div
                          initial={{ height: 0, opacity: 0 }}
                          animate={{ height: "auto", opacity: 1 }}
                          exit={{ height: 0, opacity: 0 }}
                          transition={{ duration: 0.2 }}
                          className="border-t border-emerald-700/30 overflow-hidden"
                        >
                          <div className="p-4 max-h-64 overflow-y-auto space-y-2">
                            {batch.events.slice(0, 3).map((event, eventIdx) => (
                              <div
                                key={eventIdx}
                                className="p-3 bg-slate-900/50 rounded border border-slate-700"
                              >
                                <div className="flex items-start justify-between mb-2">
                                  <div>
                                    <p className="text-xs font-semibold text-emerald-400">
                                      {event.event_type}
                                    </p>
                                    <p className="text-xs text-slate-400">
                                      Entity: {event.entity_id}
                                    </p>
                                  </div>
                                </div>
                                <pre className="text-xs text-slate-300 bg-slate-950/50 p-2 rounded overflow-x-auto">
                                  {JSON.stringify(event.payload, null, 2)}
                                </pre>
                              </div>
                            ))}
                            {batch.events.length > 3 && (
                              <p className="text-xs text-slate-500 text-center py-2">
                                + {batch.events.length - 3} more events
                              </p>
                            )}
                          </div>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </motion.div>
                ))}
            </div>
          )}
        </AnimatePresence>
      </div>

      {/* Stats */}
      {events.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-4"
        >
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
              <p className="text-xs text-slate-300 font-medium">Total Batches</p>
              <p className="text-2xl font-bold text-white mt-1">{events.length}</p>
            </div>
            <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
              <p className="text-xs text-slate-300 font-medium">Total Events</p>
              <p className="text-2xl font-bold text-white mt-1">{totalIngestedEvents}</p>
            </div>
            <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
              <p className="text-xs text-slate-300 font-medium flex items-center gap-1">
                <TrendingUp className="h-3 w-3" />
                Throughput
              </p>
              <p className="text-2xl font-bold text-blue-400 mt-1">{throughput.toFixed(1)}/s</p>
            </div>
            <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
              <p className="text-xs text-slate-300 font-medium">Success Rate</p>
              <p className="text-2xl font-bold text-emerald-400 mt-1">
                {(
                  (events.filter((e) => e.status === "success").length / events.length) *
                  100
                ).toFixed(0)}
                %
              </p>
            </div>
          </div>

          {/* Event Type Breakdown */}
          {topEventTypes.length > 0 && (
            <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
              <h4 className="text-sm font-semibold text-white mb-3">Top Event Types</h4>
              <div className="space-y-2">
                {topEventTypes.map(([eventType, count]) => (
                  <div key={eventType} className="flex items-center justify-between">
                    <span className="text-xs text-slate-300">{eventType}</span>
                    <div className="flex items-center gap-2">
                      <div className="w-32 bg-slate-700 rounded-full h-2">
                        <motion.div
                          initial={{ width: 0 }}
                          animate={{
                            width: `${(count / totalIngestedEvents) * 100}%`,
                          }}
                          className="bg-blue-500 h-2 rounded-full"
                        />
                      </div>
                      <span className="text-xs text-slate-400 w-12 text-right">{count}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </motion.div>
      )}
    </div>
  );
}
