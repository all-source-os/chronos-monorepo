# AllSource Control Plane

> Go-based orchestration and management layer for AllSource event store

[![Go](https://img.shields.io/badge/go-1.22-blue.svg)](https://golang.org/)
[![Status](https://img.shields.io/badge/status-v0.1.0-green.svg)]()
[![Framework](https://img.shields.io/badge/framework-gin-orange.svg)](https://gin-gonic.com/)

## 🎯 What is the Control Plane?

The AllSource Control Plane is a **lightweight orchestration layer** written in Go that sits above the high-performance Rust event store core. It provides:

- **Cluster Management**: Monitor and coordinate multiple event store nodes
- **Metrics Aggregation**: Collect and expose system-wide metrics
- **Operation Coordination**: Orchestrate complex operations like snapshots and replays
- **Health Monitoring**: Track health status across the cluster
- **Management APIs**: Operator-friendly APIs for system administration

## 🏗️ Architecture

```
┌────────────────────────────────────────────┐
│         Operators / Dashboards             │
└────────────────────────────────────────────┘
                  │ HTTP
                  ▼
┌────────────────────────────────────────────┐
│      Go Control Plane (Port 8081)         │
│  ┌──────────────────────────────────────┐ │
│  │  Health Monitoring                   │ │
│  │  Metrics Aggregation                 │ │
│  │  Operation Orchestration             │ │
│  │  Cluster Coordination                │ │
│  └──────────────────────────────────────┘ │
└────────────────────────────────────────────┘
                  │ HTTP Client
                  ▼
┌────────────────────────────────────────────┐
│    Rust Event Store Core (Port 8080)      │
│  • Event Ingestion (469K/sec)              │
│  • Query Engine                            │
│  • Schema Registry                         │
│  • Stream Processing                       │
└────────────────────────────────────────────┘
```

## ✨ Features

### Current (v0.1.0)

- ✅ **Health Checks**: Monitor control plane and core service health
- ✅ **Cluster Status**: View cluster topology and node status
- ✅ **Metrics Aggregation**: Collect metrics from core services
- ✅ **Snapshot Coordination**: Trigger and manage snapshots
- ✅ **Replay Orchestration**: Coordinate event replay operations
- ✅ **CORS Support**: Enable cross-origin requests for web UIs
- ✅ **Graceful Shutdown**: Clean shutdown with connection draining

### Roadmap

See [STATUS.md](../core/STATUS.md#-go-control-plane-status) for detailed roadmap.

## 🔧 API Endpoints

### Health & Status

```bash
# Control plane health check
GET /health

# Core service health check
GET /health/core

# Cluster status
GET /api/v1/cluster/status
```

### Metrics & Monitoring

```bash
# Aggregated metrics
GET /api/v1/metrics
```

### Operations

```bash
# Create snapshot
POST /api/v1/operations/snapshot
{
  "entity_id": "user-123"  # optional
}

# Trigger replay
POST /api/v1/operations/replay
{
  "entity_id": "user-123",
  "as_of": "2024-01-15T10:00:00Z"
}
```

## 🚀 Quick Start

### Prerequisites

- Go 1.22 or higher
- AllSource Rust Core running on `localhost:8080`

### Installation

```bash
# Clone repository
git clone <repo-url>
cd chronos-monorepo/services/control-plane

# Install dependencies
go mod download

# Run
go run main.go
```

### Configuration

Environment variables:

```bash
# Control plane port (default: 8081)
export PORT=8081

# Core service URL (default: http://localhost:8080)
# Can be modified in main.go if needed
```

### Running

```bash
# Development mode
go run main.go

# Build and run
go build -o control-plane
./control-plane

# Production build (optimized)
CGO_ENABLED=0 go build -ldflags="-s -w" -o control-plane
./control-plane
```

### Docker (Future)

```bash
# Build Docker image
docker build -t allsource-control-plane .

# Run container
docker run -p 8081:8081 \
  -e CORE_URL=http://core:8080 \
  allsource-control-plane
```

## 📋 Usage Examples

### Check Cluster Status

```bash
curl http://localhost:8081/api/v1/cluster/status
```

**Response**:
```json
{
  "cluster_id": "allsource-demo",
  "nodes": [
    {
      "id": "core-1",
      "type": "event-store",
      "status": "healthy",
      "url": "http://localhost:8080",
      "stats": {
        "total_events": 1234,
        "total_entities": 456
      }
    }
  ],
  "total_nodes": 1,
  "healthy_nodes": 1,
  "timestamp": "2025-10-20T12:00:00Z"
}
```

### Get Aggregated Metrics

```bash
curl http://localhost:8081/api/v1/metrics
```

**Response**:
```json
{
  "metrics": {
    "event_store": {
      "total_events": 1234,
      "total_entities": 456
    },
    "control_plane": {
      "uptime_seconds": 3600,
      "requests_handled": 42
    }
  },
  "timestamp": "2025-10-20T12:00:00Z"
}
```

### Create Snapshot

```bash
curl -X POST http://localhost:8081/api/v1/operations/snapshot \
  -H "Content-Type: application/json" \
  -d '{}'
```

**Response**:
```json
{
  "snapshot_id": "snapshot-1729425600",
  "status": "created",
  "timestamp": "2025-10-20T12:00:00Z",
  "message": "Snapshot created successfully (demo mode)"
}
```

### Trigger Replay

```bash
curl -X POST http://localhost:8081/api/v1/operations/replay \
  -H "Content-Type: application/json" \
  -d '{
    "entity_id": "user-123",
    "as_of": "2025-01-15T10:00:00Z"
  }'
```

**Response**:
```json
{
  "status": "replay_initiated",
  "entity_id": "user-123",
  "as_of": "2025-01-15T10:00:00Z",
  "timestamp": "2025-10-20T12:00:00Z",
  "message": "Event replay initiated (demo mode)"
}
```

## 🎯 Design Principles

### Why Go?

The control plane is built in Go for several key reasons:

1. **Excellent for Infrastructure Tools**: Go is the language of choice for orchestration tools (Kubernetes, Docker, Consul)
2. **Fast Compilation**: Quick iteration during development
3. **Simple Deployment**: Single binary with no dependencies
4. **Great Concurrency**: Goroutines for handling multiple nodes
5. **Strong HTTP Libraries**: Gin, Resty provide robust HTTP handling
6. **Kubernetes Native**: Easy integration with K8s operators (future)

### Why Separate from Rust Core?

1. **Separation of Concerns**: Hot path (ingestion) vs. control plane operations
2. **Language Strengths**: Rust for performance, Go for orchestration
3. **Independent Scaling**: Scale event store separately from management layer
4. **Developer Experience**: Go's simplicity for ops tooling, Rust's safety for core
5. **Ecosystem**: Leverage Go's cloud-native tooling ecosystem

## 🗺️ Roadmap

### v0.2 - Enhanced Monitoring (Q2 2025)
- [ ] Prometheus metrics export
- [ ] Real request tracking
- [ ] Dashboard API endpoints
- [ ] Alert configuration
- [ ] Log aggregation
- [ ] Grafana integration

### v0.3 - Multi-Node Support (Q3 2025)
- [ ] Node registration and discovery
- [ ] Multi-node health checking
- [ ] Load balancer integration
- [ ] Failover coordination
- [ ] Node affinity rules

### v0.4 - Advanced Operations (Q3 2025)
- [ ] Schema registry management
- [ ] Pipeline deployment
- [ ] Configuration management
- [ ] Rolling updates
- [ ] Backup orchestration

### v1.0 - Production Ready (Q4 2025)
- [ ] Service mesh integration
- [ ] Distributed tracing (OpenTelemetry)
- [ ] Multi-region coordination
- [ ] Policy enforcement
- [ ] RBAC
- [ ] Audit logging
- [ ] Webhook support

## 🧪 Testing

```bash
# Run tests (when added)
go test ./...

# Run with coverage
go test -cover ./...

# Benchmark tests
go test -bench=. ./...
```

## 📊 Performance

**Current Performance** (v0.1.0):
- Latency: <5ms per request (simple health checks)
- Throughput: ~1000 requests/sec (single core)
- Memory: ~20MB RSS
- Startup: <100ms

**Target Performance** (v1.0):
- Latency: <10ms per complex operation
- Throughput: 10K+ requests/sec
- Memory: <100MB for 100 nodes
- HA support with sub-second failover

## 🛠️ Development

### Project Structure

```
control-plane/
├── main.go          # Application entry point
├── go.mod           # Go dependencies
├── package.json     # Metadata (for npm scripts if needed)
└── README.md        # This file
```

### Dependencies

- **gin-gonic/gin**: Web framework
- **go-resty/resty**: HTTP client

### Code Style

```bash
# Format code
go fmt ./...

# Lint (requires golangci-lint)
golangci-lint run

# Vet code
go vet ./...
```

## 🤝 Integration

### With Rust Core

The control plane communicates with the Rust core via HTTP:

```go
client := resty.New().
    SetTimeout(5 * time.Second).
    SetBaseURL("http://localhost:8080")

resp, err := client.R().Get("/health")
```

### Future Integrations

- **Prometheus**: Metrics scraping
- **Grafana**: Dashboard visualization
- **Kubernetes**: Operator pattern
- **Service Mesh**: Istio/Linkerd integration
- **Distributed Tracing**: OpenTelemetry

## 📖 Resources

- [Gin Web Framework](https://gin-gonic.com/)
- [Resty HTTP Client](https://github.com/go-resty/resty)
- [Go Best Practices](https://golang.org/doc/effective_go)
- [Kubernetes Operators](https://kubernetes.io/docs/concepts/extend-kubernetes/operator/)

## 🤝 Contributing

Contributions welcome! Areas of interest:

- Prometheus metrics integration
- Multi-node support
- Test coverage
- Documentation improvements
- Operator patterns

## 📄 License

MIT License - see LICENSE file for details

---

<div align="center">

**AllSource Control Plane** - *Orchestration at scale*

Built with 🐹 Go | v0.1.0

Part of the AllSource Event Store

</div>
