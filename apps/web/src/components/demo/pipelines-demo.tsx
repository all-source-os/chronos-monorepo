"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { Pipeline, PipelineOperator } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  BarChart2,
  Clock,
  Combine,
  Filter,
  GitBranch,
  Grid3x3,
  Layers,
  Play,
  Plus,
  Timer,
  Trash2,
  Workflow,
} from "lucide-react";
import { useState } from "react";

interface OperatorDefinition {
  type: string;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  defaultConfig: Record<string, unknown>;
}

const operatorTypes: OperatorDefinition[] = [
  {
    type: "Filter",
    name: "Filter",
    description: "Filter events based on conditions",
    icon: Filter,
    color: "from-blue-500 to-blue-600",
    defaultConfig: { condition: "event_type == 'order.created'" },
  },
  {
    type: "Map",
    name: "Map",
    description: "Transform event fields",
    icon: GitBranch,
    color: "from-purple-500 to-purple-600",
    defaultConfig: { fields: ["amount", "customer_id"] },
  },
  {
    type: "Enrich",
    name: "Enrich",
    description: "Add additional data to events",
    icon: Plus,
    color: "from-green-500 to-green-600",
    defaultConfig: { source: "customer_service", key: "customer_id" },
  },
  {
    type: "Window",
    name: "Window",
    description: "Group events by time windows",
    icon: Clock,
    color: "from-orange-500 to-orange-600",
    defaultConfig: { duration: "5m", slide: "1m" },
  },
  {
    type: "Batch",
    name: "Batch",
    description: "Batch events together",
    icon: Layers,
    color: "from-pink-500 to-pink-600",
    defaultConfig: { size: 100, timeout: "30s" },
  },
  {
    type: "Reduce",
    name: "Aggregate",
    description: "Aggregate events with functions",
    icon: BarChart2,
    color: "from-cyan-500 to-cyan-600",
    defaultConfig: { function: "sum", field: "amount", groupBy: "customer_id" },
  },
  {
    type: "Branch",
    name: "Deduplicate",
    description: "Remove duplicate events",
    icon: Combine,
    color: "from-yellow-500 to-yellow-600",
    defaultConfig: { key: "event_id", window: "1h" },
  },
  {
    type: "Filter",
    name: "Throttle",
    description: "Limit event rate",
    icon: Timer,
    color: "from-red-500 to-red-600",
    defaultConfig: { rate: 1000, per: "second" },
  },
  {
    type: "Map",
    name: "Partition",
    description: "Partition events by key",
    icon: Grid3x3,
    color: "from-indigo-500 to-indigo-600",
    defaultConfig: { key: "tenant_id", partitions: 10 },
  },
  {
    type: "Map",
    name: "Flat Map",
    description: "Flatten nested events",
    icon: Workflow,
    color: "from-teal-500 to-teal-600",
    defaultConfig: { field: "items" },
  },
];

export function PipelinesDemo() {
  const [pipelineName, setPipelineName] = useState("my-pipeline");
  const [operators, setOperators] = useState<PipelineOperator[]>([]);
  const [isCreating, setIsCreating] = useState(false);
  const [selectedOperator, setSelectedOperator] = useState<string | null>(null);
  const [createdPipeline, setCreatedPipeline] = useState<Pipeline | null>(null);

  const addOperator = (operatorDef: OperatorDefinition) => {
    const newOperator: PipelineOperator = {
      type: operatorDef.type as PipelineOperator["type"],
      config: operatorDef.defaultConfig,
    };
    setOperators([...operators, newOperator]);
    setSelectedOperator(null);
  };

  const removeOperator = (index: number) => {
    setOperators(operators.filter((_, i) => i !== index));
  };

  const createPipeline = async () => {
    setIsCreating(true);
    try {
      const pipeline: Omit<Pipeline, "id" | "created_at"> = {
        name: pipelineName,
        operators: operators,
        status: "Created",
      };

      const result = await eventStoreClient.createPipeline(pipeline);
      setCreatedPipeline(result);
    } catch (error) {
      console.error("Failed to create pipeline:", error);
      // Mock success for demo
      setCreatedPipeline({
        id: `pipeline-${Date.now()}`,
        name: pipelineName,
        operators: operators,
        status: "Running",
        created_at: new Date().toISOString(),
      });
    } finally {
      setIsCreating(false);
    }
  };

  const resetPipeline = () => {
    setOperators([]);
    setCreatedPipeline(null);
    setPipelineName("my-pipeline");
  };

  return (
    <div className="space-y-8">
      {/* Pipeline Name Input */}
      <div className="space-y-2">
        <label htmlFor="pipeline-name" className="text-sm font-medium text-slate-300">
          Pipeline Name
        </label>
        <input
          id="pipeline-name"
          type="text"
          value={pipelineName}
          onChange={(e) => setPipelineName(e.target.value)}
          className="w-full px-4 py-2 rounded-lg bg-slate-800/50 border border-slate-700 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
          placeholder="Enter pipeline name..."
        />
      </div>

      {/* Operator Selection */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-slate-200">Available Operators</h3>
        <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
          {operatorTypes.map((opDef) => (
            <motion.button
              key={opDef.name}
              onClick={() => addOperator(opDef)}
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              className={`p-4 rounded-lg border border-slate-700 hover:border-slate-600 bg-gradient-to-br ${opDef.color} transition-all`}
            >
              <opDef.icon className="h-6 w-6 text-white mb-2 mx-auto" />
              <p className="text-xs font-medium text-white text-center">{opDef.name}</p>
            </motion.button>
          ))}
        </div>
      </div>

      {/* Pipeline Builder */}
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-slate-200">Pipeline Configuration</h3>
          {operators.length > 0 && (
            <Button
              onClick={resetPipeline}
              variant="outline"
              size="sm"
              className="text-red-400 border-red-400 hover:bg-red-400/10"
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Clear All
            </Button>
          )}
        </div>

        {operators.length === 0 ? (
          <div className="bg-slate-800/30 rounded-xl border border-slate-700 p-12 text-center">
            <Workflow className="h-12 w-12 text-slate-500 mx-auto mb-4" />
            <p className="text-slate-400">
              Click operators above to build your event processing pipeline
            </p>
            <p className="text-slate-500 text-sm mt-2">
              Pipelines process events in sequence from top to bottom
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            <AnimatePresence>
              {operators.map((operator, index) => {
                const opDef = operatorTypes.find((d) => d.type === operator.type);
                return (
                  <motion.div
                    key={`${operator.type}-${index}`}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    exit={{ opacity: 0, x: 20 }}
                    className="bg-slate-800/50 rounded-lg border border-slate-700 p-4"
                  >
                    <div className="flex items-start gap-4">
                      <div className="flex-shrink-0">
                        <div
                          className={`p-3 rounded-lg bg-gradient-to-br ${opDef?.color || "from-gray-500 to-gray-600"}`}
                        >
                          {opDef && <opDef.icon className="h-5 w-5 text-white" />}
                        </div>
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center justify-between mb-2">
                          <h4 className="font-semibold text-white">
                            {index + 1}. {opDef?.name || operator.type}
                          </h4>
                          <button
                            type="button"
                            onClick={() => removeOperator(index)}
                            className="text-red-400 hover:text-red-300 transition-colors"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </div>
                        <p className="text-sm text-slate-400 mb-2">{opDef?.description}</p>
                        <div className="bg-slate-900/50 rounded p-3">
                          <pre className="text-xs text-slate-300 overflow-x-auto">
                            {JSON.stringify(operator.config, null, 2)}
                          </pre>
                        </div>
                      </div>
                    </div>
                    {index < operators.length - 1 && (
                      <div className="flex justify-center mt-3">
                        <div className="h-6 w-0.5 bg-slate-600" />
                      </div>
                    )}
                  </motion.div>
                );
              })}
            </AnimatePresence>

            {/* Create Pipeline Button */}
            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="pt-4">
              <Button
                onClick={createPipeline}
                disabled={isCreating || operators.length === 0}
                className="w-full bg-gradient-to-r from-blue-600 to-purple-600 hover:from-blue-500 hover:to-purple-500"
              >
                <Play className="h-4 w-4 mr-2" />
                {isCreating ? "Creating Pipeline..." : "Create & Run Pipeline"}
              </Button>
            </motion.div>
          </div>
        )}
      </div>

      {/* Created Pipeline Success */}
      {createdPipeline && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-green-900/20 border border-green-500/50 rounded-xl p-6"
        >
          <div className="flex items-start gap-4">
            <div className="flex-shrink-0">
              <div className="p-3 rounded-lg bg-green-500/20">
                <Play className="h-6 w-6 text-green-400" />
              </div>
            </div>
            <div className="flex-1">
              <h3 className="text-lg font-semibold text-green-400 mb-2">
                Pipeline Created Successfully!
              </h3>
              <div className="space-y-2 text-sm">
                <p className="text-slate-300">
                  <span className="text-slate-400">Pipeline ID:</span>{" "}
                  <span className="font-mono bg-slate-800/50 px-2 py-1 rounded">
                    {createdPipeline.id}
                  </span>
                </p>
                <p className="text-slate-300">
                  <span className="text-slate-400">Name:</span> {createdPipeline.name}
                </p>
                <p className="text-slate-300">
                  <span className="text-slate-400">Status:</span>{" "}
                  <span className="text-green-400">{createdPipeline.status}</span>
                </p>
                <p className="text-slate-300">
                  <span className="text-slate-400">Operators:</span>{" "}
                  {createdPipeline.operators.length}
                </p>
              </div>
            </div>
          </div>
        </motion.div>
      )}

      {/* Info Banner */}
      <div className="bg-slate-800/30 rounded-xl border border-slate-700 p-6">
        <div className="flex items-start gap-3">
          <Workflow className="h-6 w-6 text-blue-500 flex-shrink-0 mt-1" />
          <div>
            <h4 className="text-lg font-semibold text-slate-200 mb-2">
              Event Processing Pipelines from Clojure
            </h4>
            <p className="text-slate-400 text-sm leading-relaxed mb-3">
              Build composable event processing pipelines with 10 operator types. Pipelines execute
              operators in sequence to transform, filter, enrich, and aggregate event streams in
              real-time.
            </p>
            <div className="grid grid-cols-2 md:grid-cols-5 gap-2">
              {operatorTypes.map((op) => (
                <div
                  key={op.name}
                  className="text-xs text-slate-400 bg-slate-800/50 px-2 py-1 rounded"
                >
                  {op.name}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
