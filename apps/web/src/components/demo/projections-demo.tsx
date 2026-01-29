"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { Projection } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  Activity,
  BarChart3,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  Clock,
  Database,
  Filter,
  Layers,
  Pause,
  Play,
  Plus,
  RefreshCw,
  Sparkles,
  Trash2,
  TrendingUp,
  XCircle,
} from "lucide-react";
import { useEffect, useState } from "react";

interface ProjectionTemplate {
  type: "EntitySnapshot" | "EventCounter" | "Custom" | "TimeSeries" | "Funnel";
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bgColor: string;
  borderColor: string;
  sampleConfig: Record<string, unknown>;
}

const projectionTemplates: ProjectionTemplate[] = [
  {
    type: "EntitySnapshot",
    name: "Entity Snapshot",
    description: "Maintain current state of entities",
    icon: Database,
    color: "text-blue-500",
    bgColor: "bg-blue-500/10",
    borderColor: "border-blue-500/20",
    sampleConfig: {
      batch_size: 100,
      checkpoint_interval: 5000,
      parallel_processing: false,
      filters: {
        entity_type: "order",
      },
    },
  },
  {
    type: "EventCounter",
    name: "Event Counter",
    description: "Count events by type or entity",
    icon: BarChart3,
    color: "text-green-500",
    bgColor: "bg-green-500/10",
    borderColor: "border-green-500/20",
    sampleConfig: {
      batch_size: 500,
      checkpoint_interval: 10000,
      parallel_processing: true,
      filters: {
        event_types: ["OrderPlaced", "OrderCompleted"],
      },
    },
  },
  {
    type: "TimeSeries",
    name: "Time Series",
    description: "Aggregate events over time windows",
    icon: TrendingUp,
    color: "text-purple-500",
    bgColor: "bg-purple-500/10",
    borderColor: "border-purple-500/20",
    sampleConfig: {
      batch_size: 200,
      checkpoint_interval: 60000,
      parallel_processing: true,
      filters: {
        window_size: "1h",
        aggregation: "sum",
      },
    },
  },
  {
    type: "Funnel",
    name: "Funnel Analysis",
    description: "Track multi-step conversion flows",
    icon: Filter,
    color: "text-orange-500",
    bgColor: "bg-orange-500/10",
    borderColor: "border-orange-500/20",
    sampleConfig: {
      batch_size: 100,
      checkpoint_interval: 5000,
      parallel_processing: false,
      filters: {
        steps: ["OrderPlaced", "PaymentProcessed", "OrderShipped"],
        max_step_duration_hours: 24,
      },
    },
  },
  {
    type: "Custom",
    name: "Custom Projection",
    description: "Build your own projection logic",
    icon: Layers,
    color: "text-pink-500",
    bgColor: "bg-pink-500/10",
    borderColor: "border-pink-500/20",
    sampleConfig: {
      batch_size: 100,
      checkpoint_interval: 5000,
      parallel_processing: false,
      filters: {},
    },
  },
];

export function ProjectionsDemo() {
  const [projections, setProjections] = useState<Projection[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<ProjectionTemplate | null>(null);
  const [projectionName, setProjectionName] = useState("");
  const [expandedProjections, setExpandedProjections] = useState<Set<string>>(new Set());

  const fetchProjections = async () => {
    setIsLoading(true);
    try {
      const data = await eventStoreClient.listProjections();
      setProjections(data);
    } catch (error) {
      console.error("Failed to fetch projections:", error);
      setProjections([]);
    } finally {
      setIsLoading(false);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: fetchProjections is defined within component and only needs to run once on mount
  useEffect(() => {
    fetchProjections();
  }, []);

  const handleCreateProjection = async () => {
    if (!selectedTemplate || !projectionName.trim()) return;

    setIsLoading(true);
    try {
      await eventStoreClient.createProjection({
        name: projectionName,
        projection_type: selectedTemplate.type,
        status: "Created",
        config: selectedTemplate.sampleConfig,
      });
      await fetchProjections();
      setShowCreateForm(false);
      setSelectedTemplate(null);
      setProjectionName("");
    } catch (error) {
      console.error("Failed to create projection:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const handlePauseResume = async (projection: Projection) => {
    setIsLoading(true);
    try {
      if (projection.status === "Running") {
        await eventStoreClient.pauseProjection(projection.id);
      } else {
        await eventStoreClient.resumeProjection(projection.id);
      }
      await fetchProjections();
    } catch (error) {
      console.error("Failed to pause/resume projection:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const toggleExpanded = (id: string) => {
    const newExpanded = new Set(expandedProjections);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedProjections(newExpanded);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "Running":
        return <Activity className="h-4 w-4 text-green-400 animate-pulse" />;
      case "Paused":
        return <Pause className="h-4 w-4 text-yellow-400" />;
      case "Failed":
        return <XCircle className="h-4 w-4 text-red-400" />;
      case "Created":
        return <CheckCircle2 className="h-4 w-4 text-blue-400" />;
      case "Rebuilding":
        return <RefreshCw className="h-4 w-4 text-purple-400 animate-spin" />;
      default:
        return <Clock className="h-4 w-4 text-slate-400" />;
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "Running":
        return "bg-green-950/30 border-green-700/50";
      case "Paused":
        return "bg-yellow-950/30 border-yellow-700/50";
      case "Failed":
        return "bg-red-950/30 border-red-700/50";
      case "Created":
        return "bg-blue-950/30 border-blue-700/50";
      case "Rebuilding":
        return "bg-purple-950/30 border-purple-700/50";
      default:
        return "bg-slate-800/70 border-slate-700";
    }
  };

  const totalEventsProcessed = projections.reduce(
    (sum, p) => sum + (p.stats?.events_processed || 0),
    0
  );
  const activeProjections = projections.filter((p) => p.status === "Running").length;
  const avgProcessingTime =
    projections.reduce((sum, p) => sum + (p.stats?.avg_processing_time_ms || 0), 0) /
    (projections.length || 1);

  return (
    <div className="space-y-6">
      {/* Header with Actions */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Projection Builder</h2>
          <p className="text-sm text-slate-400 mt-1">
            Create materialized views from event streams
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Button
            onClick={fetchProjections}
            disabled={isLoading}
            size="sm"
            variant="outline"
            className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
          >
            <motion.div
              animate={isLoading ? { rotate: 360 } : {}}
              transition={{
                duration: 1,
                repeat: isLoading ? Number.POSITIVE_INFINITY : 0,
                ease: "linear",
              }}
            >
              <RefreshCw className="h-4 w-4" />
            </motion.div>
            <span className="ml-2">Refresh</span>
          </Button>
          <Button
            onClick={() => setShowCreateForm(!showCreateForm)}
            disabled={isLoading}
            size="sm"
            className="bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-500 hover:to-emerald-500 text-white"
          >
            <Plus className="h-4 w-4 mr-2" />
            New Projection
          </Button>
        </div>
      </div>

      {/* Stats Overview */}
      {projections.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="grid grid-cols-1 md:grid-cols-4 gap-4"
        >
          <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <p className="text-xs text-slate-300 font-medium">Total Projections</p>
            <p className="text-2xl font-bold text-white mt-1">{projections.length}</p>
          </div>
          <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <p className="text-xs text-slate-300 font-medium">Active</p>
            <p className="text-2xl font-bold text-green-400 mt-1">{activeProjections}</p>
          </div>
          <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <p className="text-xs text-slate-300 font-medium">Events Processed</p>
            <p className="text-2xl font-bold text-blue-400 mt-1">
              {totalEventsProcessed.toLocaleString()}
            </p>
          </div>
          <div className="p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <p className="text-xs text-slate-300 font-medium">Avg Processing Time</p>
            <p className="text-2xl font-bold text-purple-400 mt-1">
              {avgProcessingTime.toFixed(2)}ms
            </p>
          </div>
        </motion.div>
      )}

      {/* Create Projection Form */}
      <AnimatePresence>
        {showCreateForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.3 }}
            className="overflow-hidden"
          >
            <div className="p-6 bg-slate-800/70 rounded-lg border border-slate-600 space-y-4">
              <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                <Sparkles className="h-5 w-5 text-green-400" />
                Choose a Projection Type
              </h3>

              {/* Projection Templates */}
              <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
                {projectionTemplates.map((template) => (
                  <motion.button
                    key={template.type}
                    onClick={() => setSelectedTemplate(template)}
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    className={`p-4 rounded-lg border transition-all ${
                      selectedTemplate?.type === template.type
                        ? `${template.bgColor} ${template.borderColor}`
                        : "bg-slate-900/50 border-slate-700 hover:border-slate-600"
                    }`}
                  >
                    <template.icon
                      className={`h-8 w-8 mx-auto mb-2 ${
                        selectedTemplate?.type === template.type ? template.color : "text-slate-400"
                      }`}
                    />
                    <p
                      className={`text-xs font-semibold mb-1 ${
                        selectedTemplate?.type === template.type ? "text-white" : "text-slate-300"
                      }`}
                    >
                      {template.name}
                    </p>
                    <p className="text-xs text-slate-500">{template.description}</p>
                  </motion.button>
                ))}
              </div>

              {/* Configuration Form */}
              {selectedTemplate && (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="space-y-4 pt-4 border-t border-slate-700"
                >
                  <div>
                    <label htmlFor="projection-name" className="text-xs text-slate-300 block mb-2">
                      Projection Name
                    </label>
                    <input
                      id="projection-name"
                      type="text"
                      value={projectionName}
                      onChange={(e) => setProjectionName(e.target.value)}
                      placeholder={`e.g., ${selectedTemplate.name} - Orders`}
                      className="w-full px-4 py-2 rounded-md bg-slate-700 border border-slate-600 text-white text-sm focus:outline-none focus:ring-2 focus:ring-green-500"
                    />
                  </div>

                  <div>
                    <span className="text-xs text-slate-300 block mb-2">Configuration (JSON)</span>
                    <pre className="text-xs text-slate-300 bg-slate-900/50 p-4 rounded-lg border border-slate-700 overflow-x-auto">
                      {JSON.stringify(selectedTemplate.sampleConfig, null, 2)}
                    </pre>
                  </div>

                  <div className="flex gap-3">
                    <Button
                      onClick={handleCreateProjection}
                      disabled={isLoading || !projectionName.trim()}
                      className="bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-500 hover:to-emerald-500 text-white"
                    >
                      <Plus className="h-4 w-4 mr-2" />
                      Create Projection
                    </Button>
                    <Button
                      onClick={() => {
                        setShowCreateForm(false);
                        setSelectedTemplate(null);
                        setProjectionName("");
                      }}
                      variant="outline"
                      className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
                    >
                      Cancel
                    </Button>
                  </div>
                </motion.div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Projections List */}
      {isLoading && projections.length === 0 ? (
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
              <Database className="h-12 w-12 mx-auto text-blue-500" />
            </motion.div>
            <p className="mt-4 text-slate-400">Loading projections...</p>
          </div>
        </motion.div>
      ) : projections.length === 0 ? (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="text-center py-16 bg-slate-800/50 rounded-lg border border-slate-700"
        >
          <Database className="h-16 w-16 mx-auto text-slate-600 mb-4" />
          <h3 className="text-xl font-bold text-white mb-2">No Projections Yet</h3>
          <p className="text-slate-400 mb-6">
            Create your first projection to start building materialized views
          </p>
          <Button
            onClick={() => setShowCreateForm(true)}
            className="bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-500 hover:to-emerald-500 text-white"
          >
            <Plus className="h-4 w-4 mr-2" />
            Create First Projection
          </Button>
        </motion.div>
      ) : (
        <div className="space-y-3">
          {projections.map((projection, index) => {
            const template = projectionTemplates.find((t) => t.type === projection.projection_type);
            const isExpanded = expandedProjections.has(projection.id);

            return (
              <motion.div
                key={projection.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.05 }}
                className={`rounded-lg border ${getStatusColor(projection.status)}`}
              >
                <div className="p-4">
                  <div className="flex items-start justify-between">
                    <div className="flex items-start gap-3 flex-1">
                      {template && (
                        <div className={`p-3 rounded-lg ${template.bgColor}`}>
                          <template.icon className={`h-5 w-5 ${template.color}`} />
                        </div>
                      )}
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <h4 className="font-semibold text-white">{projection.name}</h4>
                          <span className="text-xs text-slate-400">
                            {projection.projection_type}
                          </span>
                        </div>
                        <div className="flex items-center gap-3 text-xs text-slate-400">
                          <div className="flex items-center gap-1">
                            {getStatusIcon(projection.status)}
                            <span>{projection.status}</span>
                          </div>
                          {projection.stats && (
                            <>
                              <span>•</span>
                              <span>
                                {projection.stats.events_processed.toLocaleString()} events
                              </span>
                              {projection.stats.errors > 0 && (
                                <>
                                  <span>•</span>
                                  <span className="text-red-400">
                                    {projection.stats.errors} errors
                                  </span>
                                </>
                              )}
                            </>
                          )}
                        </div>
                        {projection.stats?.avg_processing_time_ms && (
                          <p className="text-xs text-slate-500 mt-1">
                            Avg: {projection.stats.avg_processing_time_ms.toFixed(2)}ms
                          </p>
                        )}
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <Button
                        onClick={() => handlePauseResume(projection)}
                        disabled={isLoading}
                        size="sm"
                        variant="outline"
                        className={`${
                          projection.status === "Running"
                            ? "bg-yellow-600/20 border-yellow-500 text-yellow-400"
                            : "bg-green-600/20 border-green-500 text-green-400"
                        }`}
                      >
                        {projection.status === "Running" ? (
                          <Pause className="h-4 w-4" />
                        ) : (
                          <Play className="h-4 w-4" />
                        )}
                      </Button>
                      <Button
                        onClick={() => toggleExpanded(projection.id)}
                        size="sm"
                        variant="outline"
                        className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
                      >
                        {isExpanded ? (
                          <ChevronUp className="h-4 w-4" />
                        ) : (
                          <ChevronDown className="h-4 w-4" />
                        )}
                      </Button>
                    </div>
                  </div>
                </div>

                {/* Expanded Details */}
                <AnimatePresence>
                  {isExpanded && (
                    <motion.div
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: "auto", opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      transition={{ duration: 0.2 }}
                      className="border-t border-slate-700 overflow-hidden"
                    >
                      <div className="p-4 space-y-3">
                        <div>
                          <h5 className="text-xs font-semibold text-slate-300 mb-2">
                            Configuration
                          </h5>
                          <pre className="text-xs text-slate-300 bg-slate-900/50 p-3 rounded border border-slate-700 overflow-x-auto">
                            {JSON.stringify(projection.config, null, 2)}
                          </pre>
                        </div>

                        {projection.stats && (
                          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                            <div className="p-3 bg-slate-900/50 rounded">
                              <p className="text-xs text-slate-400">Events Processed</p>
                              <p className="text-lg font-bold text-white">
                                {projection.stats.events_processed.toLocaleString()}
                              </p>
                            </div>
                            <div className="p-3 bg-slate-900/50 rounded">
                              <p className="text-xs text-slate-400">Errors</p>
                              <p className="text-lg font-bold text-red-400">
                                {projection.stats.errors}
                              </p>
                            </div>
                            {projection.stats.avg_processing_time_ms && (
                              <div className="p-3 bg-slate-900/50 rounded">
                                <p className="text-xs text-slate-400">Avg Time</p>
                                <p className="text-lg font-bold text-purple-400">
                                  {projection.stats.avg_processing_time_ms.toFixed(2)}ms
                                </p>
                              </div>
                            )}
                            {projection.stats.last_processed_timestamp && (
                              <div className="p-3 bg-slate-900/50 rounded">
                                <p className="text-xs text-slate-400">Last Processed</p>
                                <p className="text-xs font-medium text-white mt-1">
                                  {new Date(
                                    projection.stats.last_processed_timestamp
                                  ).toLocaleString()}
                                </p>
                              </div>
                            )}
                          </div>
                        )}

                        <div className="text-xs text-slate-500">
                          <p>Created: {new Date(projection.created_at).toLocaleString()}</p>
                          <p>Updated: {new Date(projection.updated_at).toLocaleString()}</p>
                        </div>
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </motion.div>
            );
          })}
        </div>
      )}
    </div>
  );
}
