# Metrics & Monitoring Implementation Guide

**Status**: In Progress (Foundation Complete)
**Target Version**: v0.6 (Rust Core) + v0.2 (Go Control Plane)
**Last Updated**: 2025-10-20

## 🎯 Overview

This document outlines the implementation of comprehensive metrics and monitoring for AllSource, spanning both the Rust event store core and the Go control plane.

## ✅ Completed Work

### 1. Prometheus Metrics Foundation (Rust Core)

**File Created**: `src/metrics.rs` (570+ lines)

#### Metrics Categories Implemented

**Event Ingestion** (4 metrics):
- `allsource_events_ingested_total` - Total events ingested
- `allsource_events_ingested_by_type` - Events by type (labeled)
- `allsource_ingestion_duration_seconds` - Ingestion latency histogram
- `allsource_ingestion_errors_total` - Ingestion errors

**Query Performance** (3 metrics):
- `allsource_queries_total` - Total queries by type
- `allsource_query_duration_seconds` - Query latency histogram
- `allsource_query_results_total` - Results returned by query type

**Storage** (5 metrics):
- `allsource_storage_events_total` - Total events in storage
- `allsource_storage_entities_total` - Total entities
- `allsource_storage_size_bytes` - Storage size
- `allsource_parquet_files_total` - Parquet file count
- `allsource_wal_segments_total` - WAL segment count

**Projections** (3 metrics):
- `allsource_projection_events_processed` - Events by projection
- `allsource_projection_errors_total` - Errors by projection
- `allsource_projection_processing_duration_seconds` - Processing latency

**Schema Registry** (3 metrics):
- `allsource_schemas_registered_total` - Total schemas
- `allsource_schema_validations_total` - Validations by subject/result
- `allsource_schema_validation_duration_seconds` - Validation latency

**Event Replay** (5 metrics):
- `allsource_replays_started_total` - Replays started
- `allsource_replays_completed_total` - Replays completed
- `allsource_replays_failed_total` - Replays failed
- `allsource_replay_events_processed` - Events replayed
- `allsource_replay_duration_seconds` - Replay duration

**Stream Pipelines** (4 metrics):
- `allsource_pipelines_registered_total` - Active pipelines
- `allsource_pipeline_events_processed` - Events by pipeline
- `allsource_pipeline_events_filtered` - Filtered events
- `allsource_pipeline_processing_duration_seconds` - Pipeline latency

**Snapshots** (3 metrics):
- `allsource_snapshots_created_total` - Snapshots created
- `allsource_snapshot_creation_duration_seconds` - Creation duration
- `allsource_snapshots_total` - Total snapshots

**Compaction** (4 metrics):
- `allsource_compactions_total` - Compactions performed
- `allsource_compaction_duration_seconds` - Compaction duration
- `allsource_compaction_files_merged` - Files merged
- `allsource_compaction_bytes_saved` - Bytes saved

**WebSocket** (4 metrics):
- `allsource_websocket_connections_active` - Active connections
- `allsource_websocket_connections_total` - Total connections
- `allsource_websocket_messages_sent` - Messages sent
- `allsource_websocket_errors_total` - WebSocket errors

**HTTP/System** (3 metrics):
- `allsource_http_requests_total` - Requests by method/endpoint/status
- `allsource_http_request_duration_seconds` - Request duration
- `allsource_http_requests_in_flight` - In-flight requests

**Total**: 49 Prometheus metrics defined and registered

#### Dependencies Added

```toml
prometheus = "0.13"
jsonschema = "0.18"
```

#### Module Integration

- ✅ Added to `src/lib.rs`
- ✅ Added to `src/main.rs`
- ✅ `MetricsRegistry` struct with singleton pattern
- ✅ All metrics pre-registered with Prometheus registry
- ✅ `encode()` method for Prometheus text format export

---

## 🚧 Remaining Work

### Phase 1: Rust Core Metrics Integration

#### 1.1 Add Metrics to EventStore

**File**: `src/store.rs`

**Changes Needed**:

```rust
use crate::metrics::MetricsRegistry;

pub struct EventStore {
    // ... existing fields ...
    metrics: Arc<MetricsRegistry>,
}

impl EventStore {
    pub fn new() -> Self {
        let metrics = MetricsRegistry::new();
        // ... rest of initialization ...
        Self {
            // ... existing fields ...
            metrics,
        }
    }

    pub fn ingest(&self, event: Event) -> Result<()> {
        // Start timer
        let timer = self.metrics.ingestion_duration_seconds.start_timer();

        // Existing ingestion logic
        let result = self.ingest_internal(event);

        // Record metrics
        match &result {
            Ok(_) => {
                self.metrics.events_ingested_total.inc();
                self.metrics.events_ingested_by_type
                    .with_label_values(&[&event.event_type])
                    .inc();
                self.metrics.storage_events_total.set(self.events.read().len() as i64);
            }
            Err(_) => {
                self.metrics.ingestion_errors_total.inc();
            }
        }

        timer.observe_duration();
        result
    }

    pub fn query(&self, req: QueryEventsRequest) -> Result<Vec<Event>> {
        let query_type = determine_query_type(&req);
        let timer = self.metrics.query_duration_seconds
            .with_label_values(&[&query_type])
            .start_timer();

        self.metrics.queries_total
            .with_label_values(&[&query_type])
            .inc();

        let results = self.query_internal(req)?;

        self.metrics.query_results_total
            .with_label_values(&[&query_type])
            .set(results.len() as i64);

        timer.observe_duration();
        Ok(results)
    }

    pub fn metrics(&self) -> Arc<MetricsRegistry> {
        Arc::clone(&self.metrics)
    }
}
```

**Estimate**: 2-3 hours

#### 1.2 Add /metrics Endpoint

**File**: `src/api.rs`

**Changes Needed**:

```rust
use crate::metrics::MetricsRegistry;

pub async fn serve(store: SharedStore, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        // ... existing routes ...
        .route("/metrics", get(prometheus_metrics))  // NEW
        // ... rest of routes ...
}

async fn prometheus_metrics(State(store): State<SharedStore>) -> impl IntoResponse {
    let metrics = store.metrics();

    match metrics.encode() {
        Ok(encoded) => Response::builder()
            .status(200)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(encoded)
            .unwrap(),
        Err(e) => Response::builder()
            .status(500)
            .body(format!("Error encoding metrics: {}", e))
            .unwrap(),
    }
}
```

**Estimate**: 30 minutes

#### 1.3 Update Projection Manager

**File**: `src/projection.rs`

**Changes Needed**:

```rust
impl ProjectionManager {
    pub fn process_event(&self, event: &Event, metrics: &MetricsRegistry) -> Result<()> {
        for projection in &self.projections {
            let timer = metrics.projection_processing_duration
                .with_label_values(&[projection.name()])
                .start_timer();

            match projection.process(event) {
                Ok(_) => {
                    metrics.projection_events_processed
                        .with_label_values(&[projection.name()])
                        .inc();
                }
                Err(e) => {
                    metrics.projection_errors_total
                        .with_label_values(&[projection.name()])
                        .inc();
                    return Err(e);
                }
            }

            timer.observe_duration();
        }
        Ok(())
    }
}
```

**Estimate**: 1 hour

---

### Phase 2: Go Control Plane Enhancements

#### 2.1 Add Prometheus Client

**File**: `services/control-plane/go.mod`

```go
require (
    // ... existing ...
    github.com/prometheus/client_golang v1.17.0
)
```

#### 2.2 Metrics Collection

**New File**: `services/control-plane/metrics.go`

```go
package main

import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promauto"
)

var (
    // Control plane metrics
    httpRequestsTotal = promauto.NewCounterVec(
        prometheus.CounterOpts{
            Name: "control_plane_http_requests_total",
            Help: "Total HTTP requests",
        },
        []string{"method", "endpoint", "status"},
    )

    httpRequestDuration = promauto.NewHistogramVec(
        prometheus.HistogramOpts{
            Name: "control_plane_http_request_duration_seconds",
            Help: "HTTP request duration",
        },
        []string{"method", "endpoint"},
    )

    coreHealthStatus = promauto.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "control_plane_core_health_status",
            Help: "Health status of core nodes (1=healthy, 0=unhealthy)",
        },
        []string{"node_id"},
    )

    coreEventsTotal = promauto.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "control_plane_core_events_total",
            Help: "Total events in core nodes",
        },
        []string{"node_id"},
    )
)
```

**Estimate**: 1 hour

#### 2.3 Metrics Middleware

**File**: `services/control-plane/main.go`

```go
func metricsMiddleware() gin.HandlerFunc {
    return func(c *gin.Context) {
        start := time.Now()
        path := c.FullPath()
        method := c.Request.Method

        c.Next()

        duration := time.Since(start).Seconds()
        status := fmt.Sprintf("%d", c.Writer.Status())

        httpRequestsTotal.WithLabelValues(method, path, status).Inc()
        httpRequestDuration.WithLabelValues(method, path).Observe(duration)
    }
}

// In setupRoutes()
cp.router.Use(metricsMiddleware())
```

**Estimate**: 30 minutes

#### 2.4 Prometheus Endpoint

**File**: `services/control-plane/main.go`

```go
import (
    "github.com/prometheus/client_golang/prometheus/promhttp"
)

func (cp *ControlPlane) setupRoutes() {
    // ... existing routes ...

    // Prometheus metrics
    cp.router.GET("/metrics", gin.WrapH(promhttp.Handler()))
}
```

**Estimate**: 15 minutes

#### 2.5 Enhanced Metrics Handler

**File**: `services/control-plane/main.go`

```go
func (cp *ControlPlane) metricsHandler(c *gin.Context) {
    // Fetch metrics from Rust core
    coreResp, err := cp.client.R().Get("/metrics")
    if err == nil {
        // Parse and aggregate core metrics
        // Update Prometheus gauges for core stats
    }

    // Fetch from Rust core stats endpoint
    statsResp, err := cp.client.R().Get("/api/v1/stats")
    if err == nil {
        var stats map[string]interface{}
        statsResp.UnmarshalJson(&stats)

        if events, ok := stats["total_events"].(float64); ok {
            coreEventsTotal.WithLabelValues("core-1").Set(events)
        }
    }

    c.JSON(http.StatusOK, gin.H{
        "control_plane": gin.H{
            "uptime_seconds": time.Since(startTime).Seconds(),
            "requests_total": // from prometheus metric
        },
        "core_nodes": gin.H{
            "events_total": // aggregated
            "queries_total": // aggregated
        },
    })
}
```

**Estimate**: 2 hours

---

### Phase 3: Dashboard API

#### 3.1 Dashboard Overview Endpoint

**File**: `services/control-plane/main.go`

```go
func (cp *ControlPlane) dashboardOverviewHandler(c *gin.Context) {
    // Aggregate all metrics into dashboard-friendly format
    c.JSON(http.StatusOK, gin.H{
        "timestamp": time.Now().UTC(),
        "cluster": gin.H{
            "total_nodes": 1,
            "healthy_nodes": 1,
            "total_events": // from core
            "ingestion_rate": // events/sec from prometheus
        },
        "performance": gin.H{
            "avg_ingestion_latency_ms": // from histogram
            "p95_query_latency_ms": // from histogram
            "events_per_second": // calculated
        },
        "health": gin.H{
            "core_status": "healthy",
            "storage_usage_percent": // calculated
            "wal_segments": // from core
        },
    })
}

// In setupRoutes()
api.GET("/dashboard/overview", cp.dashboardOverviewHandler)
api.GET("/dashboard/timeseries", cp.dashboardTimeseriesHandler)
api.GET("/dashboard/top_entities", cp.dashboardTopEntitiesHandler)
```

**Estimate**: 3 hours

#### 3.2 Time Series Data

```go
func (cp *ControlPlane) dashboardTimeseriesHandler(c *gin.Context) {
    timeRange := c.DefaultQuery("range", "1h")
    metric := c.DefaultQuery("metric", "ingestion_rate")

    // Query Prometheus for time series data
    // Return in format suitable for charting libraries
    c.JSON(http.StatusOK, gin.H{
        "metric": metric,
        "range": timeRange,
        "datapoints": []gin.H{
            {"timestamp": "2025-10-20T12:00:00Z", "value": 450000},
            {"timestamp": "2025-10-20T12:01:00Z", "value": 460000},
            // ... more datapoints
        },
    })
}
```

**Estimate**: 2 hours

---

### Phase 4: Grafana Integration

#### 4.1 Prometheus Configuration

**New File**: `prometheus.yml`

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'allsource-core'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'

  - job_name: 'allsource-control-plane'
    static_configs:
      - targets: ['localhost:8081']
    metrics_path: '/metrics'
```

#### 4.2 Grafana Dashboard JSON

**New File**: `grafana-dashboard.json`

Pre-built dashboard with panels for:
- Event ingestion rate
- Query latency (p50, p95, p99)
- Storage size growth
- Active WebSocket connections
- Projection processing time
- Schema validations
- Replay progress

**Estimate**: 4 hours

---

### Phase 5: Alerting

#### 5.1 Alert Rules

**New File**: `prometheus-alerts.yml`

```yaml
groups:
  - name: allsource_alerts
    rules:
      - alert: HighIngestionLatency
        expr: histogram_quantile(0.95, allsource_ingestion_duration_seconds) > 0.1
        for: 5m
        annotations:
          summary: "High ingestion latency detected"

      - alert: HighErrorRate
        expr: rate(allsource_ingestion_errors_total[5m]) > 0.01
        for: 5m
        annotations:
          summary: "Elevated error rate"

      - alert: StorageNearCapacity
        expr: allsource_storage_size_bytes > 10000000000
        for: 1m
        annotations:
          summary: "Storage approaching capacity"
```

**Estimate**: 2 hours

---

## 📊 Implementation Timeline

### Week 1: Rust Core Integration
- [ ] Day 1-2: Metrics integration in EventStore
- [ ] Day 3: Metrics endpoint and testing
- [ ] Day 4-5: Projection and pipeline metrics

### Week 2: Go Control Plane
- [ ] Day 1: Prometheus client setup
- [ ] Day 2: Metrics middleware and endpoint
- [ ] Day 3: Enhanced metrics aggregation
- [ ] Day 4-5: Dashboard API endpoints

### Week 3: Visualization & Monitoring
- [ ] Day 1-2: Grafana dashboards
- [ ] Day 3: Alert rules
- [ ] Day 4-5: Documentation and testing

**Total Estimate**: 15-18 days

---

## 🧪 Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_metrics_increment() {
        let metrics = MetricsRegistry::new();
        metrics.events_ingested_total.inc();
        assert_eq!(metrics.events_ingested_total.get(), 1);
    }

    #[test]
    fn test_metrics_encode() {
        let metrics = MetricsRegistry::new();
        let encoded = metrics.encode().unwrap();
        assert!(encoded.contains("allsource_events_ingested_total"));
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_metrics_endpoint() {
    let store = EventStore::new();
    // Ingest some events
    // Call /metrics endpoint
    // Verify prometheus format
}
```

### Load Tests
- Verify metrics overhead is <1% of request time
- Ensure no memory leaks from metric labels
- Test cardinality limits

---

## 📈 Success Criteria

✅ **Rust Core**:
- [ ] All 49 metrics actively collecting data
- [ ] `/metrics` endpoint returns valid Prometheus format
- [ ] Metrics overhead <1ms per request
- [ ] No memory leaks after 1M events

✅ **Go Control Plane**:
- [ ] Prometheus endpoint functional
- [ ] Request tracking middleware active
- [ ] Dashboard API returns real-time data
- [ ] Health checks include metrics status

✅ **Integration**:
- [ ] Grafana dashboard visualizes all key metrics
- [ ] Alerts fire correctly on thresholds
- [ ] 15-second scrape interval maintained
- [ ] Historical data retained for 30 days

---

## 🔗 Related Documentation

- [STATUS.md](STATUS.md) - Overall project status
- [README.md](README.md) - Getting started guide
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)

---

## 📝 Notes

- Metric labels should be low-cardinality (avoid entity IDs)
- Use histograms for latency, counters for totals
- Consider PromQL query performance
- Plan for metric retention (default: 15 days)
- Document all custom metrics

---

**Next Steps**:
1. Complete EventStore metrics integration
2. Add /metrics endpoint
3. Update Go control plane with Prometheus
4. Build Grafana dashboards

**Blockers**: None

**Status**: Foundation complete, integration in progress
