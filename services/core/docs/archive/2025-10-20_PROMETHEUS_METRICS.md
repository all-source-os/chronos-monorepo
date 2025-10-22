# Prometheus Metrics Guide

AllSource Event Store v0.6 includes comprehensive Prometheus metrics for monitoring and observability.

## Endpoints

### Rust Core Service (Port 8080)

**Metrics Endpoint:** `http://localhost:8080/metrics`

Returns Prometheus text format with all core service metrics.

### Go Control Plane (Port 8081)

**Metrics Endpoint:** `http://localhost:8081/metrics`

Returns Prometheus text format with control plane metrics.

**JSON Metrics (Legacy):** `http://localhost:8081/api/v1/metrics/json`

Returns aggregated metrics in JSON format for backward compatibility.

---

## Rust Core Metrics

### Event Ingestion

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_events_ingested_total` | Counter | Total events ingested | - |
| `allsource_events_ingested_by_type` | Counter | Events by type | `event_type` |
| `allsource_ingestion_duration_seconds` | Histogram | Ingestion latency | - |
| `allsource_ingestion_errors_total` | Counter | Ingestion errors | - |

**Example PromQL Queries:**
```promql
# Ingestion rate (events/sec)
rate(allsource_events_ingested_total[5m])

# P95 ingestion latency
histogram_quantile(0.95, rate(allsource_ingestion_duration_seconds_bucket[5m]))

# Top 5 event types
topk(5, sum by (event_type) (allsource_events_ingested_by_type))
```

### Query Performance

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_queries_total` | Counter | Total queries | `query_type` |
| `allsource_query_duration_seconds` | Histogram | Query latency | `query_type` |
| `allsource_query_results_total` | Counter | Results returned | `query_type` |

Query types: `entity`, `type`, `full_scan`

**Example PromQL Queries:**
```promql
# Query throughput by type
sum by (query_type) (rate(allsource_queries_total[5m]))

# P99 query latency by type
histogram_quantile(0.99, sum by (query_type, le) (
  rate(allsource_query_duration_seconds_bucket[5m])
))

# Average results per query
rate(allsource_query_results_total[5m]) / rate(allsource_queries_total[5m])
```

### Storage

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_storage_events_total` | Gauge | Total events stored | - |
| `allsource_storage_entities_total` | Gauge | Total entities | - |
| `allsource_storage_size_bytes` | Gauge | Storage size | - |
| `allsource_parquet_files_total` | Gauge | Parquet files | - |
| `allsource_wal_segments_total` | Gauge | WAL segments | - |

**Example PromQL Queries:**
```promql
# Storage growth rate (bytes/hour)
rate(allsource_storage_size_bytes[1h]) * 3600

# Events per entity
allsource_storage_events_total / allsource_storage_entities_total

# Storage health check
allsource_wal_segments_total > 100  # Alert if WAL segments exceed threshold
```

### Projections

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_projections_total` | Gauge | Registered projections | - |
| `allsource_projection_events_processed` | Counter | Events processed | `projection_name` |
| `allsource_projection_errors_total` | Counter | Processing errors | `projection_name` |
| `allsource_projection_processing_duration_seconds` | Histogram | Per-projection duration | `projection_name` |
| `allsource_projection_duration_seconds` | Histogram | Overall manager duration | - |

**Example PromQL Queries:**
```promql
# Projection throughput
sum by (projection_name) (rate(allsource_projection_events_processed[5m]))

# Projection error rate
sum by (projection_name) (rate(allsource_projection_errors_total[5m]))

# P95 projection processing time
histogram_quantile(0.95, sum by (projection_name, le) (
  rate(allsource_projection_processing_duration_seconds_bucket[5m])
))
```

### Pipelines

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_pipelines_registered_total` | Gauge | Registered pipelines | - |
| `allsource_pipeline_events_processed` | Counter | Events processed | `pipeline_id`, `pipeline_name` |
| `allsource_pipeline_events_filtered` | Counter | Events filtered out | `pipeline_id`, `pipeline_name` |
| `allsource_pipeline_errors_total` | Counter | Pipeline errors | `pipeline_name` |
| `allsource_pipeline_processing_duration_seconds` | Histogram | Per-pipeline duration | `pipeline_id`, `pipeline_name` |
| `allsource_pipeline_duration_seconds` | Histogram | Overall manager duration | - |

**Example PromQL Queries:**
```promql
# Pipeline throughput
sum by (pipeline_name) (rate(allsource_pipeline_events_processed[5m]))

# Filter ratio (filtered vs processed)
sum by (pipeline_name) (rate(allsource_pipeline_events_filtered[5m])) /
sum by (pipeline_name) (rate(allsource_pipeline_events_processed[5m]))

# Pipeline error rate
sum by (pipeline_name) (rate(allsource_pipeline_errors_total[5m]))
```

### Schema Registry

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_schemas_registered_total` | Counter | Schemas registered | - |
| `allsource_schema_validations_total` | Counter | Validations performed | `subject`, `result` |
| `allsource_schema_validation_duration_seconds` | Histogram | Validation duration | - |

**Example PromQL Queries:**
```promql
# Schema validation rate
rate(allsource_schema_validations_total[5m])

# Validation success rate
sum(rate(allsource_schema_validations_total{result="success"}[5m])) /
sum(rate(allsource_schema_validations_total[5m]))
```

### Event Replay

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_replays_started_total` | Counter | Replays started | - |
| `allsource_replays_completed_total` | Counter | Replays completed | - |
| `allsource_replays_failed_total` | Counter | Replays failed | - |
| `allsource_replay_events_processed` | Counter | Events replayed | - |
| `allsource_replay_duration_seconds` | Histogram | Replay duration | - |

**Example PromQL Queries:**
```promql
# Replay success rate
rate(allsource_replays_completed_total[1h]) /
rate(allsource_replays_started_total[1h])

# Average replay duration
rate(allsource_replay_duration_seconds_sum[5m]) /
rate(allsource_replay_duration_seconds_count[5m])
```

### Snapshots

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_snapshots_created_total` | Counter | Snapshots created | - |
| `allsource_snapshot_creation_duration_seconds` | Histogram | Creation duration | - |
| `allsource_snapshots_total` | Gauge | Active snapshots | - |

**Example PromQL Queries:**
```promql
# Snapshot creation rate
rate(allsource_snapshots_created_total[1h])

# P99 snapshot creation time
histogram_quantile(0.99, rate(allsource_snapshot_creation_duration_seconds_bucket[5m]))
```

### Compaction

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_compactions_total` | Counter | Compactions performed | - |
| `allsource_compaction_duration_seconds` | Histogram | Compaction duration | - |
| `allsource_compaction_files_merged` | Counter | Files merged | - |
| `allsource_compaction_bytes_saved` | Counter | Bytes saved | - |

**Example PromQL Queries:**
```promql
# Compaction efficiency (bytes saved per compaction)
rate(allsource_compaction_bytes_saved[1h]) /
rate(allsource_compactions_total[1h])
```

### WebSocket

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_websocket_connections_active` | Gauge | Active connections | - |
| `allsource_websocket_connections_total` | Counter | Total connections | - |
| `allsource_websocket_messages_sent` | Counter | Messages sent | - |
| `allsource_websocket_errors_total` | Counter | WebSocket errors | - |

**Example PromQL Queries:**
```promql
# Connection churn rate
rate(allsource_websocket_connections_total[5m])

# Message throughput per connection
rate(allsource_websocket_messages_sent[5m]) /
allsource_websocket_connections_active
```

### HTTP/System

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `allsource_http_requests_total` | Counter | HTTP requests | `method`, `endpoint`, `status` |
| `allsource_http_request_duration_seconds` | Histogram | Request duration | `method`, `endpoint` |
| `allsource_http_requests_in_flight` | Gauge | Active requests | - |

**Example PromQL Queries:**
```promql
# Request rate by endpoint
sum by (endpoint) (rate(allsource_http_requests_total[5m]))

# Error rate (4xx + 5xx)
sum(rate(allsource_http_requests_total{status=~"[45].."}[5m])) /
sum(rate(allsource_http_requests_total[5m]))

# P95 latency by endpoint
histogram_quantile(0.95, sum by (endpoint, le) (
  rate(allsource_http_request_duration_seconds_bucket[5m])
))
```

---

## Go Control Plane Metrics

### HTTP Requests

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `control_plane_http_requests_total` | Counter | Total HTTP requests | `method`, `path`, `status` |
| `control_plane_http_request_duration_seconds` | Histogram | Request duration | `method`, `path` |
| `control_plane_http_requests_in_flight` | Gauge | Active requests | - |

**Example PromQL Queries:**
```promql
# Request rate
rate(control_plane_http_requests_total[5m])

# P99 latency
histogram_quantile(0.99, rate(control_plane_http_request_duration_seconds_bucket[5m]))
```

### Core Service Health

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `control_plane_core_health_checks_total` | Counter | Health checks performed | `status` |
| `control_plane_core_health_check_duration_seconds` | Histogram | Health check duration | - |

**Example PromQL Queries:**
```promql
# Health check success rate
rate(control_plane_core_health_checks_total{status="success"}[5m]) /
rate(control_plane_core_health_checks_total[5m])
```

### Operations

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `control_plane_snapshot_operations_total` | Counter | Snapshot operations | - |
| `control_plane_replay_operations_total` | Counter | Replay operations | - |

### System

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `control_plane_uptime_seconds` | Gauge | Service uptime | - |

---

## Prometheus Configuration

### Scrape Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  # AllSource Rust Core
  - job_name: 'allsource-core'
    static_configs:
      - targets: ['localhost:8080']
    scrape_interval: 15s
    metrics_path: /metrics

  # AllSource Control Plane
  - job_name: 'allsource-control-plane'
    static_configs:
      - targets: ['localhost:8081']
    scrape_interval: 15s
    metrics_path: /metrics
```

### Alert Rules

Example alerting rules (`alerts.yml`):

```yaml
groups:
  - name: allsource
    interval: 30s
    rules:
      # High ingestion error rate
      - alert: HighIngestionErrorRate
        expr: rate(allsource_ingestion_errors_total[5m]) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High ingestion error rate"
          description: "Event ingestion error rate is {{ $value }} errors/sec"

      # Slow queries
      - alert: SlowQueries
        expr: |
          histogram_quantile(0.95,
            rate(allsource_query_duration_seconds_bucket[5m])
          ) > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Slow query performance"
          description: "P95 query latency is {{ $value }}s"

      # Pipeline errors
      - alert: PipelineErrors
        expr: rate(allsource_pipeline_errors_total[5m]) > 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Pipeline processing errors"
          description: "Pipeline {{ $labels.pipeline_name }} has errors"

      # Control plane down
      - alert: ControlPlaneDown
        expr: up{job="allsource-control-plane"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Control plane is down"
          description: "Control plane has been down for 1 minute"
```

---

## Grafana Integration

### Data Source Setup

1. Add Prometheus data source in Grafana
2. URL: `http://prometheus:9090` (or your Prometheus address)
3. Access: `Server` or `Browser` depending on your setup

### Dashboard Import

We provide pre-built Grafana dashboards in the `/grafana` directory:

- **allsource-overview.json** - System overview dashboard
- **allsource-ingestion.json** - Event ingestion metrics
- **allsource-queries.json** - Query performance
- **allsource-pipelines.json** - Pipeline monitoring
- **allsource-operations.json** - Operational metrics

### Custom Dashboard Panels

**Event Ingestion Rate:**
```promql
rate(allsource_events_ingested_total[5m])
```

**Query Latency Heatmap:**
```promql
sum(rate(allsource_query_duration_seconds_bucket[5m])) by (le, query_type)
```

**Storage Growth:**
```promql
allsource_storage_size_bytes
```

**Active Pipelines:**
```promql
allsource_pipelines_registered_total
```

---

## Performance Impact

Metrics collection has minimal performance impact:

- **Overhead:** < 1% CPU, < 10MB memory
- **Latency:** < 0.1ms per operation (counters/gauges)
- **Latency:** < 0.5ms per operation (histograms)
- **Cardinality:** ~100-500 time series (depending on configuration)

### Optimization Tips

1. **Reduce Histogram Buckets:** Modify bucket ranges if needed
2. **Limit Label Cardinality:** Avoid unbounded labels (e.g., entity IDs)
3. **Adjust Scrape Interval:** 15-60s is recommended
4. **Use Recording Rules:** Pre-aggregate complex queries

---

## Troubleshooting

### Metrics Not Appearing

1. Check endpoint is accessible: `curl http://localhost:8080/metrics`
2. Verify Prometheus scrape config
3. Check Prometheus targets page: `http://prometheus:9090/targets`

### High Cardinality

```promql
# Check series count
count(allsource_events_ingested_by_type)

# Find high-cardinality metrics
topk(10, count by (__name__) ({__name__=~".+"}))
```

### Missing Labels

Ensure labels are properly set in metric definitions. Check code for:
```rust
metrics.pipeline_events_processed
    .with_label_values(&[&pipeline_id, &pipeline_name])
    .inc();
```

---

## Next Steps

1. **Set up Prometheus server** - See [Prometheus docs](https://prometheus.io/docs/introduction/overview/)
2. **Configure Grafana** - Import our dashboards or create custom ones
3. **Set up alerts** - Use our example alert rules as a starting point
4. **Monitor continuously** - Review metrics regularly to understand system behavior

**Version:** 0.6.0
**Last Updated:** 2025-10
**Metrics Count:** 55+ metrics across both services
