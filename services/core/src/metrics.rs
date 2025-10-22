use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::sync::Arc;

/// Centralized metrics registry for AllSource
pub struct MetricsRegistry {
    /// Prometheus registry
    registry: Registry,

    // Event ingestion metrics
    pub events_ingested_total: IntCounter,
    pub events_ingested_by_type: IntCounterVec,
    pub ingestion_duration_seconds: Histogram,
    pub ingestion_errors_total: IntCounter,

    // Query metrics
    pub queries_total: IntCounterVec,
    pub query_duration_seconds: HistogramVec,
    pub query_results_total: IntCounterVec,

    // Storage metrics
    pub storage_events_total: IntGauge,
    pub storage_entities_total: IntGauge,
    pub storage_size_bytes: IntGauge,
    pub parquet_files_total: IntGauge,
    pub wal_segments_total: IntGauge,

    // Projection metrics
    pub projections_total: IntGauge,
    pub projection_events_processed: IntCounterVec,
    pub projection_errors_total: IntCounterVec,
    pub projection_processing_duration: HistogramVec,
    pub projection_duration_seconds: Histogram,

    // Schema registry metrics (v0.5)
    pub schemas_registered_total: IntCounter,
    pub schema_validations_total: IntCounterVec,
    pub schema_validation_duration: Histogram,

    // Replay metrics (v0.5)
    pub replays_started_total: IntCounter,
    pub replays_completed_total: IntCounter,
    pub replays_failed_total: IntCounter,
    pub replay_events_processed: IntCounter,
    pub replay_duration_seconds: Histogram,

    // Pipeline metrics (v0.5)
    pub pipelines_registered_total: IntGauge,
    pub pipeline_events_processed: IntCounterVec,
    pub pipeline_events_filtered: IntCounterVec,
    pub pipeline_errors_total: IntCounterVec,
    pub pipeline_processing_duration: HistogramVec,
    pub pipeline_duration_seconds: Histogram,

    // Snapshot metrics
    pub snapshots_created_total: IntCounter,
    pub snapshot_creation_duration: Histogram,
    pub snapshots_total: IntGauge,

    // Compaction metrics
    pub compactions_total: IntCounter,
    pub compaction_duration_seconds: Histogram,
    pub compaction_files_merged: IntCounter,
    pub compaction_bytes_saved: IntCounter,

    // WebSocket metrics
    pub websocket_connections_active: IntGauge,
    pub websocket_connections_total: IntCounter,
    pub websocket_messages_sent: IntCounter,
    pub websocket_errors_total: IntCounter,

    // System metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_requests_in_flight: IntGauge,
}

impl MetricsRegistry {
    pub fn new() -> Arc<Self> {
        let registry = Registry::new();

        // Event ingestion metrics
        let events_ingested_total = IntCounter::with_opts(Opts::new(
            "allsource_events_ingested_total",
            "Total number of events ingested",
        ))
        .unwrap();

        let events_ingested_by_type = IntCounterVec::new(
            Opts::new(
                "allsource_events_ingested_by_type",
                "Events ingested by type",
            ),
            &["event_type"],
        )
        .unwrap();

        let ingestion_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "allsource_ingestion_duration_seconds",
            "Event ingestion duration in seconds",
        ))
        .unwrap();

        let ingestion_errors_total = IntCounter::with_opts(Opts::new(
            "allsource_ingestion_errors_total",
            "Total number of ingestion errors",
        ))
        .unwrap();

        // Query metrics
        let queries_total = IntCounterVec::new(
            Opts::new("allsource_queries_total", "Total number of queries"),
            &["query_type"],
        )
        .unwrap();

        let query_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "allsource_query_duration_seconds",
                "Query duration in seconds",
            ),
            &["query_type"],
        )
        .unwrap();

        let query_results_total = IntCounterVec::new(
            Opts::new(
                "allsource_query_results_total",
                "Total number of events returned by queries",
            ),
            &["query_type"],
        )
        .unwrap();

        // Storage metrics
        let storage_events_total = IntGauge::with_opts(Opts::new(
            "allsource_storage_events_total",
            "Total number of events in storage",
        ))
        .unwrap();

        let storage_entities_total = IntGauge::with_opts(Opts::new(
            "allsource_storage_entities_total",
            "Total number of entities in storage",
        ))
        .unwrap();

        let storage_size_bytes = IntGauge::with_opts(Opts::new(
            "allsource_storage_size_bytes",
            "Total storage size in bytes",
        ))
        .unwrap();

        let parquet_files_total = IntGauge::with_opts(Opts::new(
            "allsource_parquet_files_total",
            "Number of Parquet files",
        ))
        .unwrap();

        let wal_segments_total = IntGauge::with_opts(Opts::new(
            "allsource_wal_segments_total",
            "Number of WAL segments",
        ))
        .unwrap();

        // Projection metrics
        let projection_events_processed = IntCounterVec::new(
            Opts::new(
                "allsource_projection_events_processed",
                "Events processed by projections",
            ),
            &["projection_name"],
        )
        .unwrap();

        let projection_errors_total = IntCounterVec::new(
            Opts::new(
                "allsource_projection_errors_total",
                "Total projection errors",
            ),
            &["projection_name"],
        )
        .unwrap();

        let projection_processing_duration = HistogramVec::new(
            HistogramOpts::new(
                "allsource_projection_processing_duration_seconds",
                "Projection processing duration",
            ),
            &["projection_name"],
        )
        .unwrap();

        let projections_total = IntGauge::with_opts(Opts::new(
            "allsource_projections_total",
            "Number of registered projections",
        ))
        .unwrap();

        let projection_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "allsource_projection_duration_seconds",
            "Overall projection manager processing duration",
        ))
        .unwrap();

        // Schema registry metrics (v0.5)
        let schemas_registered_total = IntCounter::with_opts(Opts::new(
            "allsource_schemas_registered_total",
            "Total number of schemas registered",
        ))
        .unwrap();

        let schema_validations_total = IntCounterVec::new(
            Opts::new(
                "allsource_schema_validations_total",
                "Schema validations by result",
            ),
            &["subject", "result"],
        )
        .unwrap();

        let schema_validation_duration = Histogram::with_opts(HistogramOpts::new(
            "allsource_schema_validation_duration_seconds",
            "Schema validation duration",
        ))
        .unwrap();

        // Replay metrics (v0.5)
        let replays_started_total = IntCounter::with_opts(Opts::new(
            "allsource_replays_started_total",
            "Total replays started",
        ))
        .unwrap();

        let replays_completed_total = IntCounter::with_opts(Opts::new(
            "allsource_replays_completed_total",
            "Total replays completed",
        ))
        .unwrap();

        let replays_failed_total = IntCounter::with_opts(Opts::new(
            "allsource_replays_failed_total",
            "Total replays failed",
        ))
        .unwrap();

        let replay_events_processed = IntCounter::with_opts(Opts::new(
            "allsource_replay_events_processed",
            "Events processed during replays",
        ))
        .unwrap();

        let replay_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "allsource_replay_duration_seconds",
            "Replay duration",
        ))
        .unwrap();

        // Pipeline metrics (v0.5)
        let pipelines_registered_total = IntGauge::with_opts(Opts::new(
            "allsource_pipelines_registered_total",
            "Number of registered pipelines",
        ))
        .unwrap();

        let pipeline_events_processed = IntCounterVec::new(
            Opts::new(
                "allsource_pipeline_events_processed",
                "Events processed by pipelines",
            ),
            &["pipeline_id", "pipeline_name"],
        )
        .unwrap();

        let pipeline_events_filtered = IntCounterVec::new(
            Opts::new(
                "allsource_pipeline_events_filtered",
                "Events filtered by pipelines",
            ),
            &["pipeline_id", "pipeline_name"],
        )
        .unwrap();

        let pipeline_processing_duration = HistogramVec::new(
            HistogramOpts::new(
                "allsource_pipeline_processing_duration_seconds",
                "Pipeline processing duration",
            ),
            &["pipeline_id", "pipeline_name"],
        )
        .unwrap();

        let pipeline_errors_total = IntCounterVec::new(
            Opts::new(
                "allsource_pipeline_errors_total",
                "Total pipeline errors",
            ),
            &["pipeline_name"],
        )
        .unwrap();

        let pipeline_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "allsource_pipeline_duration_seconds",
            "Overall pipeline manager processing duration",
        ))
        .unwrap();

        // Snapshot metrics
        let snapshots_created_total = IntCounter::with_opts(Opts::new(
            "allsource_snapshots_created_total",
            "Total snapshots created",
        ))
        .unwrap();

        let snapshot_creation_duration = Histogram::with_opts(HistogramOpts::new(
            "allsource_snapshot_creation_duration_seconds",
            "Snapshot creation duration",
        ))
        .unwrap();

        let snapshots_total = IntGauge::with_opts(Opts::new(
            "allsource_snapshots_total",
            "Total number of snapshots",
        ))
        .unwrap();

        // Compaction metrics
        let compactions_total = IntCounter::with_opts(Opts::new(
            "allsource_compactions_total",
            "Total compactions performed",
        ))
        .unwrap();

        let compaction_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "allsource_compaction_duration_seconds",
            "Compaction duration",
        ))
        .unwrap();

        let compaction_files_merged = IntCounter::with_opts(Opts::new(
            "allsource_compaction_files_merged",
            "Files merged during compaction",
        ))
        .unwrap();

        let compaction_bytes_saved = IntCounter::with_opts(Opts::new(
            "allsource_compaction_bytes_saved",
            "Bytes saved by compaction",
        ))
        .unwrap();

        // WebSocket metrics
        let websocket_connections_active = IntGauge::with_opts(Opts::new(
            "allsource_websocket_connections_active",
            "Active WebSocket connections",
        ))
        .unwrap();

        let websocket_connections_total = IntCounter::with_opts(Opts::new(
            "allsource_websocket_connections_total",
            "Total WebSocket connections",
        ))
        .unwrap();

        let websocket_messages_sent = IntCounter::with_opts(Opts::new(
            "allsource_websocket_messages_sent",
            "WebSocket messages sent",
        ))
        .unwrap();

        let websocket_errors_total = IntCounter::with_opts(Opts::new(
            "allsource_websocket_errors_total",
            "WebSocket errors",
        ))
        .unwrap();

        // System metrics
        let http_requests_total = IntCounterVec::new(
            Opts::new("allsource_http_requests_total", "Total HTTP requests"),
            &["method", "endpoint", "status"],
        )
        .unwrap();

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "allsource_http_request_duration_seconds",
                "HTTP request duration",
            ),
            &["method", "endpoint"],
        )
        .unwrap();

        let http_requests_in_flight = IntGauge::with_opts(Opts::new(
            "allsource_http_requests_in_flight",
            "HTTP requests currently being processed",
        ))
        .unwrap();

        // Register all metrics
        registry
            .register(Box::new(events_ingested_total.clone()))
            .unwrap();
        registry
            .register(Box::new(events_ingested_by_type.clone()))
            .unwrap();
        registry
            .register(Box::new(ingestion_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(ingestion_errors_total.clone()))
            .unwrap();

        registry.register(Box::new(queries_total.clone())).unwrap();
        registry
            .register(Box::new(query_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(query_results_total.clone()))
            .unwrap();

        registry
            .register(Box::new(storage_events_total.clone()))
            .unwrap();
        registry
            .register(Box::new(storage_entities_total.clone()))
            .unwrap();
        registry
            .register(Box::new(storage_size_bytes.clone()))
            .unwrap();
        registry
            .register(Box::new(parquet_files_total.clone()))
            .unwrap();
        registry
            .register(Box::new(wal_segments_total.clone()))
            .unwrap();

        registry
            .register(Box::new(projection_events_processed.clone()))
            .unwrap();
        registry
            .register(Box::new(projection_errors_total.clone()))
            .unwrap();
        registry
            .register(Box::new(projection_processing_duration.clone()))
            .unwrap();
        registry
            .register(Box::new(projections_total.clone()))
            .unwrap();
        registry
            .register(Box::new(projection_duration_seconds.clone()))
            .unwrap();

        registry
            .register(Box::new(schemas_registered_total.clone()))
            .unwrap();
        registry
            .register(Box::new(schema_validations_total.clone()))
            .unwrap();
        registry
            .register(Box::new(schema_validation_duration.clone()))
            .unwrap();

        registry
            .register(Box::new(replays_started_total.clone()))
            .unwrap();
        registry
            .register(Box::new(replays_completed_total.clone()))
            .unwrap();
        registry
            .register(Box::new(replays_failed_total.clone()))
            .unwrap();
        registry
            .register(Box::new(replay_events_processed.clone()))
            .unwrap();
        registry
            .register(Box::new(replay_duration_seconds.clone()))
            .unwrap();

        registry
            .register(Box::new(pipelines_registered_total.clone()))
            .unwrap();
        registry
            .register(Box::new(pipeline_events_processed.clone()))
            .unwrap();
        registry
            .register(Box::new(pipeline_events_filtered.clone()))
            .unwrap();
        registry
            .register(Box::new(pipeline_processing_duration.clone()))
            .unwrap();
        registry
            .register(Box::new(pipeline_errors_total.clone()))
            .unwrap();
        registry
            .register(Box::new(pipeline_duration_seconds.clone()))
            .unwrap();

        registry
            .register(Box::new(snapshots_created_total.clone()))
            .unwrap();
        registry
            .register(Box::new(snapshot_creation_duration.clone()))
            .unwrap();
        registry
            .register(Box::new(snapshots_total.clone()))
            .unwrap();

        registry
            .register(Box::new(compactions_total.clone()))
            .unwrap();
        registry
            .register(Box::new(compaction_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(compaction_files_merged.clone()))
            .unwrap();
        registry
            .register(Box::new(compaction_bytes_saved.clone()))
            .unwrap();

        registry
            .register(Box::new(websocket_connections_active.clone()))
            .unwrap();
        registry
            .register(Box::new(websocket_connections_total.clone()))
            .unwrap();
        registry
            .register(Box::new(websocket_messages_sent.clone()))
            .unwrap();
        registry
            .register(Box::new(websocket_errors_total.clone()))
            .unwrap();

        registry
            .register(Box::new(http_requests_total.clone()))
            .unwrap();
        registry
            .register(Box::new(http_request_duration_seconds.clone()))
            .unwrap();
        registry
            .register(Box::new(http_requests_in_flight.clone()))
            .unwrap();

        Arc::new(Self {
            registry,
            events_ingested_total,
            events_ingested_by_type,
            ingestion_duration_seconds,
            ingestion_errors_total,
            queries_total,
            query_duration_seconds,
            query_results_total,
            storage_events_total,
            storage_entities_total,
            storage_size_bytes,
            parquet_files_total,
            wal_segments_total,
            projection_events_processed,
            projection_errors_total,
            projection_processing_duration,
            projections_total,
            projection_duration_seconds,
            schemas_registered_total,
            schema_validations_total,
            schema_validation_duration,
            replays_started_total,
            replays_completed_total,
            replays_failed_total,
            replay_events_processed,
            replay_duration_seconds,
            pipelines_registered_total,
            pipeline_events_processed,
            pipeline_events_filtered,
            pipeline_processing_duration,
            pipeline_errors_total,
            pipeline_duration_seconds,
            snapshots_created_total,
            snapshot_creation_duration,
            snapshots_total,
            compactions_total,
            compaction_duration_seconds,
            compaction_files_merged,
            compaction_bytes_saved,
            websocket_connections_active,
            websocket_connections_total,
            websocket_messages_sent,
            websocket_errors_total,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
        })
    }

    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Encode metrics in Prometheus text format
    pub fn encode(&self) -> Result<String, Box<dyn std::error::Error>> {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

// Note: Clone and Default are intentionally NOT implemented for MetricsRegistry.
// Use Arc<MetricsRegistry> to share the same registry across the application.
// Creating multiple registries would result in duplicate metrics which is incorrect.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let metrics = MetricsRegistry::new();
        assert_eq!(metrics.events_ingested_total.get(), 0);
        assert_eq!(metrics.storage_events_total.get(), 0);
    }

    #[test]
    fn test_event_ingestion_metrics() {
        let metrics = MetricsRegistry::new();

        // Increment ingestion counter
        metrics.events_ingested_total.inc();
        assert_eq!(metrics.events_ingested_total.get(), 1);

        // Increment by type
        metrics.events_ingested_by_type
            .with_label_values(&["user.created"])
            .inc();
        assert_eq!(
            metrics.events_ingested_by_type
                .with_label_values(&["user.created"])
                .get(),
            1
        );

        // Record duration
        metrics.ingestion_duration_seconds.observe(0.1);
    }

    #[test]
    fn test_query_metrics() {
        let metrics = MetricsRegistry::new();

        // Increment query counter
        metrics.queries_total
            .with_label_values(&["entity_id"])
            .inc();
        assert_eq!(
            metrics.queries_total
                .with_label_values(&["entity_id"])
                .get(),
            1
        );

        // Record query duration
        metrics.query_duration_seconds
            .with_label_values(&["entity_id"])
            .observe(0.05);

        // Record query results
        metrics.query_results_total
            .with_label_values(&["entity_id"])
            .inc_by(10);
    }

    #[test]
    fn test_storage_metrics() {
        let metrics = MetricsRegistry::new();

        // Set storage metrics
        metrics.storage_events_total.set(1000);
        assert_eq!(metrics.storage_events_total.get(), 1000);

        metrics.storage_entities_total.set(50);
        assert_eq!(metrics.storage_entities_total.get(), 50);

        metrics.storage_size_bytes.set(1024 * 1024);
        assert_eq!(metrics.storage_size_bytes.get(), 1024 * 1024);

        metrics.parquet_files_total.set(5);
        metrics.wal_segments_total.set(3);
    }

    #[test]
    fn test_projection_metrics() {
        let metrics = MetricsRegistry::new();

        // Set projections total
        metrics.projections_total.set(3);
        assert_eq!(metrics.projections_total.get(), 3);

        // Process events in projection
        metrics.projection_events_processed
            .with_label_values(&["user_snapshot"])
            .inc_by(100);

        // Record processing duration
        metrics.projection_processing_duration
            .with_label_values(&["user_snapshot"])
            .observe(0.2);

        // Record errors
        metrics.projection_errors_total
            .with_label_values(&["user_snapshot"])
            .inc();
    }

    #[test]
    fn test_schema_metrics() {
        let metrics = MetricsRegistry::new();

        // Register schema
        metrics.schemas_registered_total.inc();
        assert_eq!(metrics.schemas_registered_total.get(), 1);

        // Validation success - requires both subject and result labels
        metrics.schema_validations_total
            .with_label_values(&["user.schema", "success"])
            .inc();

        // Validation failure
        metrics.schema_validations_total
            .with_label_values(&["order.schema", "failure"])
            .inc();

        // Record validation duration
        metrics.schema_validation_duration.observe(0.01);
    }

    #[test]
    fn test_replay_metrics() {
        let metrics = MetricsRegistry::new();

        // Start replay
        metrics.replays_started_total.inc();
        assert_eq!(metrics.replays_started_total.get(), 1);

        // Process events
        metrics.replay_events_processed.inc_by(500);
        assert_eq!(metrics.replay_events_processed.get(), 500);

        // Complete replay
        metrics.replays_completed_total.inc();
        assert_eq!(metrics.replays_completed_total.get(), 1);

        // Record duration
        metrics.replay_duration_seconds.observe(5.5);
    }

    #[test]
    fn test_pipeline_metrics() {
        let metrics = MetricsRegistry::new();

        // Register pipeline
        metrics.pipelines_registered_total.set(2);
        assert_eq!(metrics.pipelines_registered_total.get(), 2);

        // Process events - requires both pipeline_id and pipeline_name labels
        metrics.pipeline_events_processed
            .with_label_values(&["pipeline-1", "filter_pipeline"])
            .inc_by(250);

        // Record errors - only requires pipeline_name
        metrics.pipeline_errors_total
            .with_label_values(&["filter_pipeline"])
            .inc();

        // Record duration - requires both pipeline_id and pipeline_name labels
        metrics.pipeline_processing_duration
            .with_label_values(&["pipeline-1", "filter_pipeline"])
            .observe(0.15);
    }

    #[test]
    fn test_metrics_encode() {
        let metrics = MetricsRegistry::new();

        // Add some data
        metrics.events_ingested_total.inc_by(100);
        metrics.storage_events_total.set(1000);

        // Encode to Prometheus format
        let encoded = metrics.encode().unwrap();

        // Verify output contains metrics
        assert!(encoded.contains("events_ingested_total"));
        assert!(encoded.contains("storage_events_total"));
    }

    #[test]
    fn test_metrics_default() {
        let metrics = MetricsRegistry::new();
        assert_eq!(metrics.events_ingested_total.get(), 0);
    }

    #[test]
    fn test_websocket_metrics() {
        let metrics = MetricsRegistry::new();

        // Connect client
        metrics.websocket_connections_active.inc();
        assert_eq!(metrics.websocket_connections_active.get(), 1);

        // Total connections
        metrics.websocket_connections_total.inc();

        // Broadcast message
        metrics.websocket_messages_sent.inc_by(10);
        assert_eq!(metrics.websocket_messages_sent.get(), 10);

        // Disconnect client
        metrics.websocket_connections_active.dec();
        assert_eq!(metrics.websocket_connections_active.get(), 0);

        // Record error
        metrics.websocket_errors_total.inc();
    }

    #[test]
    fn test_compaction_metrics() {
        let metrics = MetricsRegistry::new();

        // Start compaction
        metrics.compactions_total.inc();
        assert_eq!(metrics.compactions_total.get(), 1);

        // Record duration
        metrics.compaction_duration_seconds.observe(5.2);

        // Files merged
        metrics.compaction_files_merged.inc_by(5);

        // Bytes saved
        metrics.compaction_bytes_saved.inc_by(1024 * 1024);
    }

    #[test]
    fn test_snapshot_metrics() {
        let metrics = MetricsRegistry::new();

        // Create snapshot
        metrics.snapshots_created_total.inc();
        assert_eq!(metrics.snapshots_created_total.get(), 1);

        // Record duration
        metrics.snapshot_creation_duration.observe(0.5);

        // Total snapshots
        metrics.snapshots_total.set(10);
        assert_eq!(metrics.snapshots_total.get(), 10);
    }

    #[test]
    fn test_http_metrics() {
        let metrics = MetricsRegistry::new();

        // Record request
        metrics.http_requests_total
            .with_label_values(&["GET", "/api/events", "200"])
            .inc();

        // Record duration
        metrics.http_request_duration_seconds
            .with_label_values(&["GET", "/api/events"])
            .observe(0.025);

        // In-flight requests
        metrics.http_requests_in_flight.inc();
        assert_eq!(metrics.http_requests_in_flight.get(), 1);

        metrics.http_requests_in_flight.dec();
        assert_eq!(metrics.http_requests_in_flight.get(), 0);
    }
}
