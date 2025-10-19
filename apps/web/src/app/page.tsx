"use client";

import { useState, useEffect } from "react";
import axios from "axios";
import { format } from "date-fns";

const CORE_API = "http://localhost:8080";
const CONTROL_API = "http://localhost:8081";

interface Event {
  id: string;
  event_type: string;
  entity_id: string;
  payload: any;
  timestamp: string;
  version: number;
}

interface Stats {
  total_events: number;
  total_entities: number;
  total_event_types: number;
  total_ingested: number;
}

export default function Home() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [events, setEvents] = useState<Event[]>([]);
  const [loading, setLoading] = useState(false);
  const [entityId, setEntityId] = useState("");
  const [eventType, setEventType] = useState("");

  // Demo data generation
  const [demoEntityId] = useState(`user-${Math.floor(Math.random() * 1000)}`);

  useEffect(() => {
    fetchStats();
    fetchEvents();
  }, []);

  const fetchStats = async () => {
    try {
      const response = await axios.get(`${CORE_API}/api/v1/stats`);
      setStats(response.data);
    } catch (error) {
      console.error("Failed to fetch stats:", error);
    }
  };

  const fetchEvents = async () => {
    try {
      setLoading(true);
      const params: any = {};
      if (entityId) params.entity_id = entityId;
      if (eventType) params.event_type = eventType;

      const response = await axios.get(`${CORE_API}/api/v1/events/query`, {
        params,
      });
      setEvents(response.data.events || []);
    } catch (error) {
      console.error("Failed to fetch events:", error);
    } finally {
      setLoading(false);
    }
  };

  const ingestDemoEvent = async () => {
    try {
      const eventTypes = [
        "user.created",
        "user.updated",
        "order.placed",
        "payment.processed",
      ];
      const randomType =
        eventTypes[Math.floor(Math.random() * eventTypes.length)];

      await axios.post(`${CORE_API}/api/v1/events`, {
        event_type: randomType,
        entity_id: demoEntityId,
        payload: {
          action: randomType,
          timestamp: new Date().toISOString(),
          data: {
            value: Math.floor(Math.random() * 1000),
            status: "active",
          },
        },
      });

      await fetchStats();
      await fetchEvents();
    } catch (error) {
      console.error("Failed to ingest event:", error);
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900 p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="text-center mb-12">
          <h1 className="text-6xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-purple-400 mb-4">
            AllSource
          </h1>
          <p className="text-xl text-gray-300">
            AI-Native Event Store • Time-Travel Through Data
          </p>
        </div>

        {/* Stats Dashboard */}
        {stats && (
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
            <StatCard
              title="Total Events"
              value={stats.total_events}
              icon="📊"
            />
            <StatCard
              title="Entities"
              value={stats.total_entities}
              icon="🎯"
            />
            <StatCard
              title="Event Types"
              value={stats.total_event_types}
              icon="🏷️"
            />
            <StatCard
              title="Ingested"
              value={stats.total_ingested}
              icon="⚡"
            />
          </div>
        )}

        {/* Demo Actions */}
        <div className="bg-white/10 backdrop-blur-lg rounded-lg p-6 mb-8">
          <h2 className="text-2xl font-semibold text-white mb-4">
            Demo Controls
          </h2>
          <div className="flex gap-4">
            <button
              onClick={ingestDemoEvent}
              className="bg-gradient-to-r from-blue-500 to-purple-500 hover:from-blue-600 hover:to-purple-600 text-white px-6 py-3 rounded-lg font-medium transition-all"
            >
              Ingest Demo Event for {demoEntityId}
            </button>
            <button
              onClick={fetchEvents}
              className="bg-white/20 hover:bg-white/30 text-white px-6 py-3 rounded-lg font-medium transition-all"
            >
              Refresh Events
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className="bg-white/10 backdrop-blur-lg rounded-lg p-6 mb-8">
          <h2 className="text-2xl font-semibold text-white mb-4">
            Query Events
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <input
              type="text"
              placeholder="Entity ID (e.g., user-123)"
              value={entityId}
              onChange={(e) => setEntityId(e.target.value)}
              className="bg-white/20 text-white placeholder-gray-400 px-4 py-2 rounded-lg border border-white/30 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <input
              type="text"
              placeholder="Event Type (e.g., user.created)"
              value={eventType}
              onChange={(e) => setEventType(e.target.value)}
              className="bg-white/20 text-white placeholder-gray-400 px-4 py-2 rounded-lg border border-white/30 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              onClick={fetchEvents}
              disabled={loading}
              className="bg-gradient-to-r from-cyan-500 to-blue-500 hover:from-cyan-600 hover:to-blue-600 text-white px-6 py-2 rounded-lg font-medium transition-all disabled:opacity-50"
            >
              {loading ? "Loading..." : "Search"}
            </button>
          </div>
        </div>

        {/* Events List */}
        <div className="bg-white/10 backdrop-blur-lg rounded-lg p-6">
          <h2 className="text-2xl font-semibold text-white mb-4">
            Events ({events.length})
          </h2>
          <div className="space-y-4 max-h-[600px] overflow-y-auto">
            {events.length === 0 ? (
              <p className="text-gray-400 text-center py-8">
                No events found. Click "Ingest Demo Event" to create some!
              </p>
            ) : (
              events.map((event) => (
                <EventCard key={event.id} event={event} />
              ))
            )}
          </div>
        </div>
      </div>
    </main>
  );
}

function StatCard({
  title,
  value,
  icon,
}: {
  title: string;
  value: number;
  icon: string;
}) {
  return (
    <div className="bg-white/10 backdrop-blur-lg rounded-lg p-6 border border-white/20">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-gray-400 text-sm mb-1">{title}</p>
          <p className="text-3xl font-bold text-white">{value}</p>
        </div>
        <div className="text-4xl">{icon}</div>
      </div>
    </div>
  );
}

function EventCard({ event }: { event: Event }) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="bg-white/5 border border-white/10 rounded-lg p-4 hover:bg-white/10 transition-all">
      <div
        className="flex justify-between items-start cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <span className="bg-blue-500/20 text-blue-300 px-3 py-1 rounded-full text-sm font-medium">
              {event.event_type}
            </span>
            <span className="text-gray-400 text-sm">
              Entity: {event.entity_id}
            </span>
          </div>
          <p className="text-gray-300 text-sm">
            {format(new Date(event.timestamp), "PPpp")}
          </p>
        </div>
        <div className="text-gray-400">
          {expanded ? "▼" : "▶"}
        </div>
      </div>

      {expanded && (
        <div className="mt-4 pt-4 border-t border-white/10">
          <pre className="bg-black/30 p-4 rounded text-sm text-gray-300 overflow-x-auto">
            {JSON.stringify(event.payload, null, 2)}
          </pre>
          <div className="mt-2 text-xs text-gray-500">
            ID: {event.id} • Version: {event.version}
          </div>
        </div>
      )}
    </div>
  );
}
