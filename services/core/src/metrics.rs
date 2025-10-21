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

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new().as_ref().clone()
    }
}

impl Clone for MetricsRegistry {
    fn clone(&self) -> Self {
        // Note: This creates a new metrics registry. In practice, you should
        // use Arc<MetricsRegistry> to share the same registry across the application
        Self::new().as_ref().clone()
    }
}
