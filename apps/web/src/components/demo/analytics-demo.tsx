"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { AnalyticsQuery } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { motion } from "motion/react";
import {
  Activity,
  ArrowRight,
  BarChart,
  LineChart,
  PieChart,
  TrendingUp,
  Users,
} from "lucide-react";
import { useState } from "react";

interface AnalyticsResultData {
  metric: string;
  value: number | string;
  unit?: string;
  change?: number;
  data?: Array<{ label: string; value: number }>;
}

export function AnalyticsDemo() {
  const [isLoading, setIsLoading] = useState(false);
  const [selectedAnalytic, setSelectedAnalytic] = useState<string | null>(null);
  const [analyticsResults, setAnalyticsResults] = useState<AnalyticsResultData[]>([]);

  const runTimeSeriesAnalytics = async () => {
    setIsLoading(true);
    setSelectedAnalytic("timeseries");
    try {
      const query: AnalyticsQuery = {
        metric: "timeseries",
        event_type: "order.created",
        time_range: {
          from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
          to: new Date().toISOString(),
        },
        granularity: "day",
        aggregation: "count",
      };

      const result = await eventStoreClient.runAnalytics(query);

      // Mock result for demo purposes
      const mockData = [
        { label: "Day 1", value: 145 },
        { label: "Day 2", value: 167 },
        { label: "Day 3", value: 189 },
        { label: "Day 4", value: 203 },
        { label: "Day 5", value: 178 },
        { label: "Day 6", value: 195 },
        { label: "Day 7", value: 221 },
      ];

      setAnalyticsResults([
        {
          metric: "Time Series Analysis",
          value: "7 days",
          unit: "analyzed",
          change: 12.5,
          data: mockData,
        },
        {
          metric: "Total Events",
          value: mockData.reduce((sum, d) => sum + d.value, 0),
          unit: "events",
        },
        {
          metric: "Average per Day",
          value: Math.round(mockData.reduce((sum, d) => sum + d.value, 0) / mockData.length),
          unit: "events/day",
        },
      ]);
    } catch (error) {
      console.error("Analytics failed:", error);
      setAnalyticsResults([
        {
          metric: "Error",
          value: error instanceof Error ? error.message : "Failed to run analytics",
          unit: "Please ensure backend services are running",
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const runFunnelAnalytics = async () => {
    setIsLoading(true);
    setSelectedAnalytic("funnel");
    try {
      const query: AnalyticsQuery = {
        metric: "funnel",
        funnel_steps: ["product.viewed", "cart.added", "checkout.started", "order.created"],
        time_range: {
          from: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
          to: new Date().toISOString(),
        },
      };

      await eventStoreClient.runAnalytics(query);

      // Mock funnel data
      const funnelData = [
        { label: "Product Viewed", value: 1000 },
        { label: "Added to Cart", value: 450 },
        { label: "Started Checkout", value: 320 },
        { label: "Completed Order", value: 215 },
      ];

      setAnalyticsResults([
        {
          metric: "Funnel Analysis",
          value: "4 steps",
          unit: "analyzed",
          data: funnelData,
        },
        {
          metric: "Conversion Rate",
          value: "21.5%",
          unit: "end-to-end",
        },
        {
          metric: "Drop-off Rate",
          value: "55%",
          unit: "at cart stage",
        },
      ]);
    } catch (error) {
      console.error("Funnel analytics failed:", error);
      setAnalyticsResults([
        {
          metric: "Error",
          value: error instanceof Error ? error.message : "Failed to run funnel analytics",
          unit: "Please ensure backend services are running",
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const runCohortAnalytics = async () => {
    setIsLoading(true);
    setSelectedAnalytic("cohort");
    try {
      const query: AnalyticsQuery = {
        metric: "cohort",
        cohort_definition: {
          event_type: "user.registered",
          return_event_type: "user.login",
          time_window_days: 30,
        },
        time_range: {
          from: new Date(Date.now() - 90 * 24 * 60 * 60 * 1000).toISOString(),
          to: new Date().toISOString(),
        },
      };

      await eventStoreClient.runAnalytics(query);

      // Mock cohort data
      setAnalyticsResults([
        {
          metric: "Cohort Analysis",
          value: "3 cohorts",
          unit: "analyzed",
        },
        {
          metric: "Week 1 Retention",
          value: "68%",
          unit: "returning users",
        },
        {
          metric: "Week 4 Retention",
          value: "42%",
          unit: "returning users",
        },
        {
          metric: "Average Retention",
          value: "55%",
          unit: "across cohorts",
          change: 8.3,
        },
      ]);
    } catch (error) {
      console.error("Cohort analytics failed:", error);
      setAnalyticsResults([
        {
          metric: "Error",
          value: error instanceof Error ? error.message : "Failed to run cohort analytics",
          unit: "Please ensure backend services are running",
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const runAggregationAnalytics = async () => {
    setIsLoading(true);
    setSelectedAnalytic("aggregation");
    try {
      const query: AnalyticsQuery = {
        metric: "summary",
        event_type: "order.created",
        aggregation: "sum",
        field: "amount",
        time_range: {
          from: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
          to: new Date().toISOString(),
        },
      };

      await eventStoreClient.runAnalytics(query);

      // Mock aggregation data
      setAnalyticsResults([
        {
          metric: "Total Revenue",
          value: "$124,580",
          unit: "last 30 days",
          change: 15.2,
        },
        {
          metric: "Average Order Value",
          value: "$87.34",
          unit: "per order",
          change: 3.5,
        },
        {
          metric: "Standard Deviation",
          value: "$32.15",
          unit: "order variance",
        },
        {
          metric: "95th Percentile",
          value: "$156.20",
          unit: "order value",
        },
      ]);
    } catch (error) {
      console.error("Aggregation analytics failed:", error);
      setAnalyticsResults([
        {
          metric: "Error",
          value: error instanceof Error ? error.message : "Failed to run aggregation analytics",
          unit: "Please ensure backend services are running",
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const analyticOptions = [
    {
      id: "timeseries",
      name: "Time Series",
      description: "Analyze trends over time with multiple aggregations",
      icon: LineChart,
      color: "from-blue-500 to-cyan-500",
      action: runTimeSeriesAnalytics,
    },
    {
      id: "funnel",
      name: "Funnel Analysis",
      description: "Track user journey through conversion steps",
      icon: TrendingUp,
      color: "from-purple-500 to-pink-500",
      action: runFunnelAnalytics,
    },
    {
      id: "cohort",
      name: "Cohort Analysis",
      description: "Measure user retention across cohorts",
      icon: Users,
      color: "from-green-500 to-emerald-500",
      action: runCohortAnalytics,
    },
    {
      id: "aggregation",
      name: "Statistical Aggregations",
      description: "Compute sum, avg, stddev, percentiles and more",
      icon: BarChart,
      color: "from-orange-500 to-red-500",
      action: runAggregationAnalytics,
    },
  ];

  return (
    <div className="space-y-8">
      {/* Analytics Options */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {analyticOptions.map((option) => (
          <motion.button
            key={option.id}
            onClick={option.action}
            disabled={isLoading}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            className={`p-6 rounded-xl border transition-all text-left ${
              selectedAnalytic === option.id
                ? `bg-gradient-to-br ${option.color} border-transparent`
                : "bg-slate-800/30 border-slate-700 hover:border-slate-600"
            } ${isLoading ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}
          >
            <div className="flex items-start gap-4">
              <div
                className={`p-3 rounded-lg ${
                  selectedAnalytic === option.id ? "bg-white/20" : "bg-slate-700/50"
                }`}
              >
                <option.icon
                  className={`h-6 w-6 ${
                    selectedAnalytic === option.id ? "text-white" : "text-slate-400"
                  }`}
                />
              </div>
              <div className="flex-1">
                <h3
                  className={`text-lg font-semibold mb-1 ${
                    selectedAnalytic === option.id ? "text-white" : "text-slate-200"
                  }`}
                >
                  {option.name}
                </h3>
                <p
                  className={`text-sm ${
                    selectedAnalytic === option.id ? "text-white/80" : "text-slate-400"
                  }`}
                >
                  {option.description}
                </p>
              </div>
            </div>
          </motion.button>
        ))}
      </div>

      {/* Loading State */}
      {isLoading && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="flex items-center justify-center py-12"
        >
          <div className="flex items-center gap-3 text-slate-400">
            <Activity className="h-5 w-5 animate-spin" />
            <span>Running analytics...</span>
          </div>
        </motion.div>
      )}

      {/* Analytics Results */}
      {!isLoading && analyticsResults.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-6"
        >
          <div className="flex items-center justify-between">
            <h3 className="text-xl font-semibold text-slate-200">Analysis Results</h3>
            <span className="text-sm text-slate-400">{new Date().toLocaleTimeString()}</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {analyticsResults.map((result) => (
              <motion.div
                key={result.metric}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                className="bg-slate-800/50 backdrop-blur-sm rounded-xl border border-slate-700 p-6"
              >
                <div className="space-y-2">
                  <p className="text-sm text-slate-400">{result.metric}</p>
                  <div className="flex items-baseline gap-2">
                    <p className="text-3xl font-bold text-white">{result.value}</p>
                    {result.unit && <p className="text-sm text-slate-400">{result.unit}</p>}
                  </div>
                  {result.change && (
                    <div className="flex items-center gap-1 text-sm">
                      <ArrowRight
                        className={`h-4 w-4 ${
                          result.change > 0 ? "text-green-500" : "text-red-500"
                        } rotate-${result.change > 0 ? "-45" : "45"}`}
                      />
                      <span className={result.change > 0 ? "text-green-500" : "text-red-500"}>
                        {Math.abs(result.change)}%
                      </span>
                    </div>
                  )}
                </div>

                {/* Data visualization (simplified for demo) */}
                {result.data && result.data.length > 0 && (
                  <div className="mt-4 space-y-2">
                    {result.data.map((item) => (
                      <div key={item.label} className="flex items-center gap-2">
                        <span className="text-xs text-slate-400 w-20">{item.label}</span>
                        <div className="flex-1 bg-slate-700/30 rounded-full h-2">
                          <div
                            className="bg-gradient-to-r from-blue-500 to-purple-500 h-2 rounded-full"
                            style={{
                              width: `${(item.value / Math.max(...result.data!.map((d) => d.value))) * 100}%`,
                            }}
                          />
                        </div>
                        <span className="text-xs text-slate-300 w-12 text-right">{item.value}</span>
                      </div>
                    ))}
                  </div>
                )}
              </motion.div>
            ))}
          </div>

          {/* Features showcase */}
          <div className="bg-slate-800/30 rounded-xl border border-slate-700 p-6">
            <h4 className="text-lg font-semibold text-slate-200 mb-4">
              Available Aggregation Functions
            </h4>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {[
                "count",
                "sum",
                "avg",
                "min",
                "max",
                "stddev",
                "variance",
                "percentile",
                "distinct",
                "first",
                "last",
              ].map((func) => (
                <div
                  key={func}
                  className="bg-slate-700/30 rounded-lg px-3 py-2 text-sm text-slate-300 text-center"
                >
                  {func}
                </div>
              ))}
            </div>
          </div>
        </motion.div>
      )}

      {/* Info Banner */}
      {!selectedAnalytic && (
        <div className="bg-slate-800/30 rounded-xl border border-slate-700 p-6">
          <div className="flex items-start gap-3">
            <PieChart className="h-6 w-6 text-purple-500 flex-shrink-0 mt-1" />
            <div>
              <h4 className="text-lg font-semibold text-slate-200 mb-2">
                Advanced Analytics from Clojure Query Service
              </h4>
              <p className="text-slate-400 text-sm leading-relaxed">
                Select an analytics option above to explore powerful event analytics capabilities
                including time-series analysis, funnel tracking, cohort retention, and statistical
                aggregations. Powered by the Clojure Query Service with support for 11 aggregation
                functions.
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
