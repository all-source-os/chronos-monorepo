// Event Store Types

export interface Event {
  id: string;
  event_type: string;
  entity_id: string;
  tenant_id: string;
  payload: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  timestamp: string;
  version: number;
}

export interface CreateEventRequest {
  event_type: string;
  entity_id: string;
  tenant_id: string;
  payload: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

export interface QueryEventsParams {
  entity_id?: string;
  event_type?: string;
  from_timestamp?: string;
  to_timestamp?: string;
  as_of?: string;
  limit?: number;
}

export interface Projection {
  id: string;
  name: string;
  projection_type: "EntitySnapshot" | "EventCounter" | "Custom" | "TimeSeries" | "Funnel";
  status: "Created" | "Running" | "Paused" | "Failed" | "Stopped" | "Rebuilding";
  config: ProjectionConfig;
  stats?: ProjectionStats;
  created_at: string;
  updated_at: string;
}

export interface ProjectionConfig {
  batch_size?: number;
  checkpoint_interval?: number;
  parallel_processing?: boolean;
  filters?: Record<string, unknown>;
}

export interface ProjectionStats {
  events_processed: number;
  errors: number;
  last_processed_timestamp?: string;
  avg_processing_time_ms?: number;
}

export interface Schema {
  id: string;
  event_type: string;
  version: number;
  schema: Record<string, unknown>; // JSON Schema
  compatibility_mode: "None" | "Backward" | "Forward" | "Full";
  created_at: string;
}

export interface Snapshot {
  id: string;
  entity_id: string;
  snapshot_type: string;
  state: Record<string, unknown>;
  version: number;
  timestamp: string;
}

export interface Pipeline {
  id: string;
  name: string;
  operators: PipelineOperator[];
  status: "Created" | "Running" | "Paused" | "Failed";
  created_at: string;
}

export interface PipelineOperator {
  type: "Filter" | "Map" | "Reduce" | "Window" | "Enrich" | "Branch";
  config: Record<string, unknown>;
}

export interface AnalyticsQuery {
  metric: "frequency" | "correlation" | "summary" | "timeseries" | "funnel" | "cohort";
  event_type?: string;
  time_range?: {
    from: string;
    to: string;
  };
  granularity?: "minute" | "hour" | "day" | "week" | "month";
  aggregation?: "count" | "sum" | "avg" | "min" | "max" | "stddev" | "variance" | "percentile" | "distinct" | "first" | "last";
  field?: string;
  funnel_steps?: string[];
  cohort_definition?: {
    event_type: string;
    return_event_type: string;
    time_window_days: number;
  };
}

export interface AnalyticsResult {
  metric: string;
  data: unknown;
  timestamp: string;
  metadata?: Record<string, unknown>;
}

export interface Tenant {
  id: string;
  name: string;
  tier: "Free" | "Standard" | "Professional" | "Enterprise";
  quotas: TenantQuotas;
  created_at: string;
}

export interface TenantQuotas {
  events_per_day: number;
  storage_gb: number;
  queries_per_hour: number;
  max_projections: number;
  max_pipelines: number;
  max_api_keys: number;
}

export interface Metrics {
  ingestion_rate: number;
  query_latency_ms: number;
  active_connections: number;
  storage_used_gb: number;
  events_total: number;
  timestamp: string;
}
