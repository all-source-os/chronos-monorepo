"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { Event } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  ChevronDown,
  ChevronUp,
  Clock,
  Database,
  Download,
  Filter,
  History,
  Search,
  Sparkles,
  Zap,
} from "lucide-react";
import { useState } from "react";

interface QueryHistory {
  type: string;
  params: string;
  resultCount: number;
  duration: number;
  timestamp: number;
}

export function QueryDemo() {
  const [isQuerying, setIsQuerying] = useState(false);
  const [queryResults, setQueryResults] = useState<Event[]>([]);
  const [queryType, setQueryType] = useState<string>("");
  const [expandedResults, setExpandedResults] = useState<Set<string>>(new Set());
  const [queryDuration, setQueryDuration] = useState<number>(0);
  const [queryHistory, setQueryHistory] = useState<QueryHistory[]>([]);

  // Custom query inputs
  const [customEntityId, setCustomEntityId] = useState("order-1");
  const [customEventType, setCustomEventType] = useState("OrderPlaced");
  const [timeRangeHours, setTimeRangeHours] = useState(24);
  const [resultLimit, setResultLimit] = useState(20);

  const runQuery = async (type: "entity" | "type" | "timerange" | "custom") => {
    setIsQuerying(true);
    setQueryType(type);
    const startTime = Date.now();

    try {
      let results: Event[] = [];
      let queryParams = "";

      switch (type) {
        case "entity":
          results = await eventStoreClient.getEventsByEntity(customEntityId);
          queryParams = `entity_id: ${customEntityId}`;
          break;
        case "type":
          results = await eventStoreClient.getEventsByType(customEventType);
          queryParams = `event_type: ${customEventType}`;
          break;
        case "timerange": {
          const now = new Date();
          const hoursAgo = new Date(now.getTime() - timeRangeHours * 60 * 60 * 1000);
          results = await eventStoreClient.queryEvents({
            from_timestamp: hoursAgo.toISOString(),
            to_timestamp: now.toISOString(),
            limit: resultLimit,
          });
          queryParams = `last ${timeRangeHours}h, limit: ${resultLimit}`;
          break;
        }
        case "custom": {
          // Advanced query combining multiple filters
          const customNow = new Date();
          const customHoursAgo = new Date(customNow.getTime() - timeRangeHours * 60 * 60 * 1000);
          results = await eventStoreClient.queryEvents({
            event_type: customEventType,
            from_timestamp: customHoursAgo.toISOString(),
            to_timestamp: customNow.toISOString(),
            limit: resultLimit,
          });
          queryParams = `type: ${customEventType}, time: ${timeRangeHours}h, limit: ${resultLimit}`;
          break;
        }
      }

      const duration = Date.now() - startTime;
      setQueryDuration(duration);
      setQueryResults(results);

      // Add to history
      setQueryHistory((prev) => [
        {
          type,
          params: queryParams,
          resultCount: results.length,
          duration,
          timestamp: Date.now(),
        },
        ...prev.slice(0, 4), // Keep last 5 queries
      ]);
    } catch (error) {
      console.error("Query failed:", error);
      setQueryResults([]);
      setQueryDuration(Date.now() - startTime);
    } finally {
      setIsQuerying(false);
    }
  };

  const toggleExpanded = (eventId: string) => {
    const newExpanded = new Set(expandedResults);
    if (newExpanded.has(eventId)) {
      newExpanded.delete(eventId);
    } else {
      newExpanded.add(eventId);
    }
    setExpandedResults(newExpanded);
  };

  const exportResults = () => {
    const dataStr = JSON.stringify(queryResults, null, 2);
    const dataBlob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(dataBlob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `query-results-${Date.now()}.json`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      {/* Custom Query Inputs */}
      <div className="p-4 bg-slate-800/50 rounded-lg border border-slate-600 space-y-3">
        <h4 className="text-sm font-semibold text-white mb-3">Query Parameters</h4>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div>
            <label htmlFor="entity-id" className="text-xs text-slate-300 block mb-1">
              Entity ID
            </label>
            <input
              id="entity-id"
              type="text"
              value={customEntityId}
              onChange={(e) => setCustomEntityId(e.target.value)}
              disabled={isQuerying}
              className="w-full px-3 py-2 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="e.g., order-1, user-1"
            />
          </div>
          <div>
            <label htmlFor="event-type" className="text-xs text-slate-300 block mb-1">
              Event Type
            </label>
            <input
              id="event-type"
              type="text"
              value={customEventType}
              onChange={(e) => setCustomEventType(e.target.value)}
              disabled={isQuerying}
              className="w-full px-3 py-2 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="e.g., OrderPlaced, UserRegistered"
            />
          </div>
          <div>
            <label htmlFor="time-range" className="text-xs text-slate-300 block mb-1">
              Time Range (hours)
            </label>
            <select
              id="time-range"
              value={timeRangeHours}
              onChange={(e) => setTimeRangeHours(Number(e.target.value))}
              disabled={isQuerying}
              className="w-full px-3 py-2 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value={1}>Last 1 hour</option>
              <option value={6}>Last 6 hours</option>
              <option value={24}>Last 24 hours</option>
              <option value={168}>Last 7 days</option>
            </select>
          </div>
          <div>
            <label htmlFor="result-limit" className="text-xs text-slate-300 block mb-1">
              Result Limit
            </label>
            <select
              id="result-limit"
              value={resultLimit}
              onChange={(e) => setResultLimit(Number(e.target.value))}
              disabled={isQuerying}
              className="w-full px-3 py-2 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value={10}>10 results</option>
              <option value={20}>20 results</option>
              <option value={50}>50 results</option>
              <option value={100}>100 results</option>
            </select>
          </div>
        </div>
      </div>

      {/* Query Buttons */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <Button
          onClick={() => runQuery("entity")}
          disabled={isQuerying}
          className="h-20 bg-slate-700/50 hover:bg-slate-600/50 border-2 border-slate-600 text-white"
        >
          <motion.div
            className="flex flex-col items-center gap-1"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Database className="h-5 w-5 text-blue-400" />
            <span className="text-sm font-semibold text-white">By Entity</span>
          </motion.div>
        </Button>

        <Button
          onClick={() => runQuery("type")}
          disabled={isQuerying}
          className="h-20 bg-slate-700/50 hover:bg-slate-600/50 border-2 border-slate-600 text-white"
        >
          <motion.div
            className="flex flex-col items-center gap-1"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Filter className="h-5 w-5 text-purple-400" />
            <span className="text-sm font-semibold text-white">By Type</span>
          </motion.div>
        </Button>

        <Button
          onClick={() => runQuery("timerange")}
          disabled={isQuerying}
          className="h-20 bg-slate-700/50 hover:bg-slate-600/50 border-2 border-slate-600 text-white"
        >
          <motion.div
            className="flex flex-col items-center gap-1"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Clock className="h-5 w-5 text-green-400" />
            <span className="text-sm font-semibold text-white">Time Range</span>
          </motion.div>
        </Button>

        <Button
          onClick={() => runQuery("custom")}
          disabled={isQuerying}
          className="h-20 bg-gradient-to-br from-orange-600 to-red-600 hover:from-orange-500 hover:to-red-500 border-2 border-orange-500 text-white"
        >
          <motion.div
            className="flex flex-col items-center gap-1"
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <Zap className="h-5 w-5 text-white" />
            <span className="text-sm font-semibold text-white">Advanced</span>
          </motion.div>
        </Button>
      </div>

      {/* Loading State */}
      {isQuerying && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="flex items-center justify-center py-12"
        >
          <div className="text-center">
            <motion.div
              animate={{ rotate: 360 }}
              transition={{ duration: 1, repeat: Number.POSITIVE_INFINITY, ease: "linear" }}
            >
              <Search className="h-12 w-12 mx-auto text-blue-500" />
            </motion.div>
            <p className="mt-4 text-slate-400">Querying event store...</p>
          </div>
        </motion.div>
      )}

      {/* Results */}
      {!isQuerying && queryResults.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-4"
        >
          {/* Results Header */}
          <div className="flex items-center justify-between p-4 bg-slate-800/50 rounded-lg border border-slate-600">
            <div className="flex items-center gap-4">
              <div className="flex items-center gap-2">
                <Sparkles className="h-5 w-5 text-yellow-500" />
                <h3 className="text-lg font-semibold text-white">Query Results</h3>
              </div>
              <span className="text-sm text-slate-400">{queryResults.length} events</span>
              <span className="text-sm text-emerald-400 flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {queryDuration}ms
              </span>
            </div>
            <Button
              onClick={exportResults}
              size="sm"
              variant="outline"
              className="bg-slate-700/50 hover:bg-slate-600/50 text-white border-slate-600"
            >
              <Download className="h-4 w-4 mr-2" />
              Export JSON
            </Button>
          </div>

          {/* Results List */}
          <div className="grid grid-cols-1 gap-2 max-h-96 overflow-y-auto">
            {queryResults.map((event, index) => {
              const isExpanded = expandedResults.has(event.id);
              return (
                <motion.div
                  key={event.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.03 }}
                  className="bg-slate-800/70 rounded-lg border border-slate-600 hover:border-slate-500 transition-colors overflow-hidden"
                >
                  <button
                    type="button"
                    className="p-3 cursor-pointer text-left w-full"
                    onClick={() => toggleExpanded(event.id)}
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <h4 className="font-semibold text-white text-sm">{event.event_type}</h4>
                          <span className="text-xs text-slate-400">v{event.version}</span>
                        </div>
                        <p className="text-xs text-slate-300">
                          Entity: <span className="text-blue-400">{event.entity_id}</span>
                        </p>
                        <p className="text-xs text-slate-400 mt-1">
                          {new Date(event.timestamp).toLocaleString()}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        {isExpanded ? (
                          <ChevronUp className="h-4 w-4 text-slate-400" />
                        ) : (
                          <ChevronDown className="h-4 w-4 text-slate-400" />
                        )}
                      </div>
                    </div>
                  </button>

                  <AnimatePresence>
                    {isExpanded && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: "auto", opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="border-t border-slate-700"
                      >
                        <div className="p-3">
                          <pre className="text-xs text-slate-200 bg-slate-900/50 p-3 rounded overflow-x-auto border border-slate-700">
                            {JSON.stringify(event.payload, null, 2)}
                          </pre>
                          {event.metadata && (
                            <div className="mt-2 p-2 bg-slate-900/30 rounded text-xs text-slate-400">
                              <strong>Metadata:</strong> {JSON.stringify(event.metadata)}
                            </div>
                          )}
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </motion.div>
              );
            })}
          </div>
        </motion.div>
      )}

      {/* No Results */}
      {!isQuerying && queryType && queryResults.length === 0 && (
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-center py-12">
          <Database className="h-12 w-12 mx-auto text-slate-600 mb-3" />
          <p className="text-slate-400">No events found for this query</p>
          <p className="text-sm text-slate-500 mt-2">
            Try ingesting some events first or adjusting your filters
          </p>
        </motion.div>
      )}

      {/* Query History */}
      {queryHistory.length > 0 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="p-4 bg-slate-800/50 rounded-lg border border-slate-600"
        >
          <h4 className="text-sm font-semibold text-white mb-3 flex items-center gap-2">
            <History className="h-4 w-4 text-slate-400" />
            Recent Queries
          </h4>
          <div className="space-y-2">
            {queryHistory.map((query) => (
              <div
                key={`${query.type}-${query.timestamp}`}
                className="flex items-center justify-between text-xs p-2 bg-slate-900/30 rounded"
              >
                <div className="flex-1">
                  <span className="text-blue-400 font-medium">{query.type}</span>
                  <span className="text-slate-400 ml-2">• {query.params}</span>
                </div>
                <div className="flex items-center gap-3 text-slate-500">
                  <span>{query.resultCount} results</span>
                  <span>{query.duration}ms</span>
                </div>
              </div>
            ))}
          </div>
        </motion.div>
      )}
    </div>
  );
}
