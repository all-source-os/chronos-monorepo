"use client";

import { AnalyticsDemo } from "@/components/demo/analytics-demo";
import { EventIngestionDemo } from "@/components/demo/event-ingestion-demo";
import { MetricsDemo } from "@/components/demo/metrics-demo";
import { PipelinesDemo } from "@/components/demo/pipelines-demo";
import { ProjectionsDemo } from "@/components/demo/projections-demo";
import { QueryDemo } from "@/components/demo/query-demo";
import { SecurityDemo } from "@/components/demo/security-demo";
import { TimeTravelDemo } from "@/components/demo/time-travel-demo";
import { BlurFade, DotPattern, Ripple } from "@allsource/ui";
import { motion } from "motion/react";
import {
  BarChart3,
  Clock,
  Database,
  PieChart,
  Shield,
  Sparkles,
  Workflow,
  Zap,
} from "lucide-react";
import { useState } from "react";

const features = [
  {
    id: "events",
    title: "Event Ingestion",
    description: "High-throughput event ingestion with batch processing support",
    icon: Zap,
    color: "from-yellow-500 to-orange-500",
  },
  {
    id: "queries",
    title: "Powerful Queries",
    description: "Query by entity, type, or time range with millisecond latency",
    icon: Database,
    color: "from-blue-500 to-cyan-500",
  },
  {
    id: "metrics",
    title: "Real-time Metrics",
    description: "Monitor system performance with live dashboards",
    icon: BarChart3,
    color: "from-purple-500 to-pink-500",
  },
  {
    id: "projections",
    title: "Projections",
    description: "Build materialized views from event streams",
    icon: Database,
    color: "from-green-500 to-emerald-500",
  },
  {
    id: "security",
    title: "Enterprise Security",
    description: "Multi-tenancy, RBAC, JWT authentication, and rate limiting",
    icon: Shield,
    color: "from-red-500 to-rose-500",
  },
  {
    id: "timetravel",
    title: "Time Travel",
    description: "Query historical state at any point in time",
    icon: Clock,
    color: "from-indigo-500 to-violet-500",
  },
  {
    id: "analytics",
    title: "Event Analytics",
    description: "Advanced analytics with funnels, cohorts, and aggregations",
    icon: PieChart,
    color: "from-teal-500 to-cyan-500",
  },
  {
    id: "pipelines",
    title: "Event Pipelines",
    description: "Build composable pipelines with 10 operator types",
    icon: Workflow,
    color: "from-amber-500 to-orange-500",
  },
];

export default function DemoPage() {
  const [activeDemo, setActiveDemo] = useState<string>("events");

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-white relative overflow-hidden">
      {/* Background Effects */}
      <DotPattern className="opacity-20" />
      <div className="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-900/80 to-slate-950/90 pointer-events-none" />

      {/* Hero Section */}
      <div className="relative z-10">
        <div className="container mx-auto px-4 pt-20 pb-12">
          <BlurFade delay={0.2} inView>
            <div className="text-center max-w-4xl mx-auto mb-16 relative py-8 px-6">
              {/* Text Background for better readability */}
              <div className="absolute inset-0 bg-slate-900/40 backdrop-blur-md rounded-3xl -z-10 border border-slate-700/30" />
              <motion.div
                initial={{ scale: 0.9, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                transition={{ duration: 0.5 }}
                className="inline-block mb-6"
              >
                <div className="relative">
                  <Ripple />
                  <div className="relative z-10 bg-gradient-to-br from-blue-600 to-purple-600 p-4 rounded-2xl">
                    <Sparkles className="h-12 w-12 text-white" />
                  </div>
                </div>
              </motion.div>

              <h1 className="text-6xl md:text-7xl font-bold mb-6 bg-gradient-to-r from-white via-blue-100 to-purple-100 bg-clip-text text-transparent">
                AllSource Event Store
              </h1>
              <p className="text-xl md:text-2xl text-slate-300 mb-4">
                AI-Native Event Sourcing Platform
              </p>
              <p className="text-lg text-slate-400 max-w-2xl mx-auto">
                Experience the next generation of event streaming with real-time analytics, temporal
                queries, and enterprise-grade security
              </p>

              {/* Performance Stats */}
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.5 }}
                className="grid grid-cols-3 gap-6 mt-12 max-w-3xl mx-auto"
              >
                <div className="bg-slate-800/50 backdrop-blur-sm p-6 rounded-xl border border-slate-700">
                  <p className="text-3xl font-bold text-yellow-400">469K/s</p>
                  <p className="text-sm text-slate-400 mt-1">Events Ingested</p>
                </div>
                <div className="bg-slate-800/50 backdrop-blur-sm p-6 rounded-xl border border-slate-700">
                  <p className="text-3xl font-bold text-green-400">&lt;100ms</p>
                  <p className="text-sm text-slate-400 mt-1">Query Latency</p>
                </div>
                <div className="bg-slate-800/50 backdrop-blur-sm p-6 rounded-xl border border-slate-700">
                  <p className="text-3xl font-bold text-blue-400">80%</p>
                  <p className="text-sm text-slate-400 mt-1">Compression</p>
                </div>
              </motion.div>
            </div>
          </BlurFade>

          {/* Feature Cards */}
          <BlurFade delay={0.4} inView>
            <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-8 gap-4 mb-16">
              {features.map((feature, index) => (
                <motion.button
                  key={feature.id}
                  onClick={() => setActiveDemo(feature.id)}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.6 + index * 0.1 }}
                  whileHover={{ scale: 1.05, y: -4 }}
                  whileTap={{ scale: 0.95 }}
                  className={`p-6 rounded-xl border transition-all ${
                    activeDemo === feature.id
                      ? `bg-gradient-to-br ${feature.color} border-transparent shadow-2xl`
                      : "bg-slate-800/30 border-slate-700 hover:border-slate-600"
                  }`}
                >
                  <feature.icon
                    className={`h-8 w-8 mb-3 mx-auto ${
                      activeDemo === feature.id ? "text-white" : "text-slate-400"
                    }`}
                  />
                  <h3
                    className={`text-sm font-semibold mb-1 ${
                      activeDemo === feature.id ? "text-white" : "text-slate-300"
                    }`}
                  >
                    {feature.title}
                  </h3>
                  <p
                    className={`text-xs ${
                      activeDemo === feature.id ? "text-white/80" : "text-slate-500"
                    }`}
                  >
                    {feature.description}
                  </p>
                </motion.button>
              ))}
            </div>
          </BlurFade>

          {/* Interactive Demo Section */}
          <BlurFade delay={0.8} inView>
            <motion.div
              key={activeDemo}
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              transition={{ duration: 0.3 }}
              className="bg-slate-800/50 backdrop-blur-sm rounded-2xl border border-slate-700 p-8 md:p-12 shadow-2xl"
            >
              {activeDemo === "events" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Zap className="h-8 w-8 text-yellow-500" />
                    Live Event Ingestion
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Ingest events in real-time with automatic batching and retry logic
                  </p>
                  <EventIngestionDemo />
                </div>
              )}

              {activeDemo === "queries" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Database className="h-8 w-8 text-blue-500" />
                    Query Engine
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Execute powerful queries with sub-millisecond latency
                  </p>
                  <QueryDemo />
                </div>
              )}

              {activeDemo === "metrics" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <BarChart3 className="h-8 w-8 text-purple-500" />
                    System Metrics
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Monitor performance with real-time metrics and alerts
                  </p>
                  <MetricsDemo />
                </div>
              )}

              {activeDemo === "projections" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Database className="h-8 w-8 text-green-500" />
                    Projections
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Create materialized views from event streams with 5 projection types
                  </p>
                  <ProjectionsDemo />
                </div>
              )}

              {activeDemo === "security" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Shield className="h-8 w-8 text-red-500" />
                    Security
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Enterprise-grade security with multi-tenancy and compliance features
                  </p>
                  <SecurityDemo />
                </div>
              )}

              {activeDemo === "timetravel" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Clock className="h-8 w-8 text-indigo-500" />
                    Time Travel
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Query historical state at any point in time with temporal queries
                  </p>
                  <TimeTravelDemo />
                </div>
              )}

              {activeDemo === "analytics" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <PieChart className="h-8 w-8 text-teal-500" />
                    Event Analytics
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Advanced analytics with time series, funnels, cohorts, and 11 aggregation
                    functions
                  </p>
                  <AnalyticsDemo />
                </div>
              )}

              {activeDemo === "pipelines" && (
                <div>
                  <h2 className="text-3xl font-bold mb-2 flex items-center gap-3">
                    <Workflow className="h-8 w-8 text-amber-500" />
                    Event Pipelines
                  </h2>
                  <p className="text-slate-400 mb-8">
                    Build composable event processing pipelines with 10 operator types
                  </p>
                  <PipelinesDemo />
                </div>
              )}
            </motion.div>
          </BlurFade>
        </div>
      </div>
    </div>
  );
}
