# WebSocket Configuration Guide

Real-time event streaming between AllSource services.

---

## Overview

The Query Service connects to Core via WebSocket for real-time event streaming. This enables:
- Live event notifications without polling
- Instant projection updates
- Real-time dashboard feeds

If you see this warning, WebSocket configuration is needed:
```
allsource-query-service │ ⚠️ WebSocket config needed
```

---

## Quick Fix

Set the `CORE_WS_URL` environment variable pointing to your Core instance:

```bash
# Docker Compose / Local
export CORE_WS_URL=ws://allsource-core:3900

# Kubernetes (port differs)
export CORE_WS_URL=ws://allsource-core:3901
```

---

## Configuration Reference

### Required

| Variable | Example | Description |
|----------|---------|-------------|
| `CORE_WS_URL` | `ws://allsource-core:3900` | WebSocket URL to Core service |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `CORE_WS_ENABLED` | `true` | Enable/disable WebSocket client |
| `CORE_WS_MAX_RECONNECT_ATTEMPTS` | `10` | Max reconnection attempts before degraded mode |
| `CORE_WS_INITIAL_BACKOFF_MS` | `1000` | Initial reconnection backoff (ms) |
| `CORE_WS_MAX_BACKOFF_MS` | `30000` | Maximum backoff between retries (ms) |
| `CORE_API_KEY` | - | API key for authentication header |

---

## Deployment Examples

### Docker Compose

```yaml
services:
  core:
    image: ghcr.io/allsource/allsource-core:latest
    ports:
      - "3900:3900"

  query-service:
    image: ghcr.io/allsource/allsource-query-service:latest
    environment:
      CORE_WS_URL: ws://core:3900          # Use service name
      CORE_WS_ENABLED: "true"
      DATABASE_URL: ecto://user:pass@postgres/allsource
      SECRET_KEY_BASE: ${SECRET_KEY_BASE}
    depends_on:
      - core
      - postgres
```

### Docker Run

```bash
# Start Core first
docker run -d --name allsource-core \
  -p 3900:3900 \
  ghcr.io/allsource/allsource-core:latest

# Start Query Service with WebSocket config
docker run -d --name allsource-query-service \
  -p 3902:3902 \
  -e CORE_WS_URL=ws://allsource-core:3900 \
  -e CORE_WS_ENABLED=true \
  -e DATABASE_URL=ecto://user:pass@postgres/allsource \
  -e SECRET_KEY_BASE=$(openssl rand -hex 64) \
  --link allsource-core \
  ghcr.io/allsource/allsource-query-service:latest
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: allsource-query-service
spec:
  template:
    spec:
      containers:
        - name: query-service
          image: ghcr.io/allsource/allsource-query-service:latest
          env:
            - name: CORE_WS_URL
              value: "ws://allsource-core:3901"  # Note: port 3901 in K8s
            - name: CORE_WS_ENABLED
              value: "true"
            - name: CORE_WS_MAX_RECONNECT_ATTEMPTS
              value: "15"  # More retries in K8s
```

### Helm

The Helm chart auto-configures WebSocket from values:

```yaml
# values.yaml
core:
  service:
    wsPort: 3901

queryService:
  config:
    coreWsEnabled: true
    # CORE_WS_URL is templated automatically as:
    # ws://{{ .Release.Name }}-core:{{ .Values.core.service.wsPort }}
```

---

## How It Works

### Connection Flow

```
┌─────────────────┐         WebSocket          ┌─────────────────┐
│  Query Service  │◄──────────────────────────►│      Core       │
│    (Elixir)     │    ws://core:3900/api/     │     (Rust)      │
│                 │       v1/events/stream      │                 │
└─────────────────┘                            └─────────────────┘
        │
        │ PubSub broadcast
        ▼
┌─────────────────┐
│   Projections   │
│    LiveView     │
│   Subscribers   │
└─────────────────┘
```

### WebSocket Endpoint

Core exposes the WebSocket at:
```
ws://<host>:<port>/api/v1/events/stream
```

Query Service automatically appends the path - just provide the base URL.

### Reconnection Behavior

The WebSocket client handles disconnections gracefully:

1. **Initial connection** - Attempts to connect on startup
2. **On disconnect** - Waits `initial_backoff` (1s default)
3. **Exponential backoff** - Each retry doubles the wait time (with ±20% jitter)
4. **Max backoff** - Caps at `max_backoff` (30s default)
5. **Degraded mode** - After `max_reconnect_attempts`, stops retrying

In degraded mode, the service continues operating with HTTP polling fallback.

---

## PubSub Topics

When connected, events are broadcast to these Phoenix PubSub topics:

| Topic | Description |
|-------|-------------|
| `events:all` | All events from Core |
| `events:{entity_id}` | Events for a specific entity |
| `events:type:{event_type}` | Events of a specific type |

### Subscribing in Elixir

```elixir
# Subscribe to all events
Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

# Subscribe to specific entity
Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:user-123")

# Handle incoming events
def handle_info({:event, event}, socket) do
  # Process event...
  {:noreply, socket}
end
```

---

## Troubleshooting

### "WebSocket config needed" warning

The Query Service started but `CORE_WS_URL` is not set or empty.

**Fix:** Set the environment variable:
```bash
export CORE_WS_URL=ws://your-core-host:3900
```

### "Connection refused" errors

Core is not running or not reachable at the configured URL.

**Debug steps:**
```bash
# 1. Check Core is running
curl http://allsource-core:3900/health

# 2. Verify WebSocket endpoint exists
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: test" \
  http://allsource-core:3900/api/v1/events/stream

# 3. Check network connectivity
docker exec allsource-query-service ping allsource-core
```

### "Max reconnection attempts reached"

The service couldn't connect after multiple retries and entered degraded mode.

**Possible causes:**
- Core service is down or restarting
- Network partition between services
- Firewall blocking WebSocket connections

**Fix:** Restart the Query Service after Core is healthy:
```bash
docker restart allsource-query-service
```

### Using wrong port

| Environment | HTTP Port | WebSocket Port |
|-------------|-----------|----------------|
| Docker Compose | 3900 | 3900 |
| Kubernetes | 3900 | 3901 |

Kubernetes separates ports for better service routing. Check your deployment target.

### Protocol mismatch

Always use `ws://` or `wss://` protocol prefix, never `http://`:

```bash
# Correct
CORE_WS_URL=ws://allsource-core:3900

# Wrong - will fail
CORE_WS_URL=http://allsource-core:3900
```

For TLS/SSL in production:
```bash
CORE_WS_URL=wss://allsource-core.example.com:443
```

---

## Health Check

The Query Service health endpoint includes WebSocket status:

```bash
curl http://localhost:3902/api/health | jq
```

Response includes:
```json
{
  "status": "healthy",
  "components": {
    "database": "connected",
    "core_websocket": "connected",
    "core_http": "connected"
  }
}
```

If WebSocket is disconnected:
```json
{
  "status": "degraded",
  "components": {
    "database": "connected",
    "core_websocket": "disconnected",
    "core_http": "connected"
  }
}
```

---

## Disabling WebSocket

If you don't need real-time updates, disable the WebSocket client:

```bash
CORE_WS_ENABLED=false
```

The Query Service will use HTTP polling for event updates instead.

---

## Production Recommendations

1. **Use TLS** - Always use `wss://` in production
2. **Set API key** - Configure `CORE_API_KEY` for authentication
3. **Increase retries** - Set `CORE_WS_MAX_RECONNECT_ATTEMPTS=20` for resilience
4. **Monitor health** - Alert on `core_websocket: disconnected` status
5. **Log verbosity** - Set `RUST_LOG=info` on Core to debug connection issues

---

## Related Documentation

- [Docker Deployment](../deployment/DOCKER.md) - Container configuration
- [Quick Start](./QUICK_START.md) - Running the full stack locally
- [API Reference](../current/API_REFERENCE.md) - Core API endpoints
