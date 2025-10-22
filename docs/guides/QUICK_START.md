# AllSource - Quick Start Guide

## ⚡ 60-Second Setup

### Step 1: Install Dependencies (One-Time Setup)

```bash
# Install Bun if you don't have it
curl -fsSL https://bun.sh/install | bash

# Install all project dependencies
bun install

# Install Go dependencies
cd services/control-plane && go mod download && cd ../..
```

### Step 2: Start Services (3 Terminals)

**Terminal 1 - Event Store Core:**
```bash
cd services/core
cargo run --release
# Wait for: "🚀 AllSource Core listening on 0.0.0.0:8080"
```

**Terminal 2 - Control Plane:**
```bash
cd services/control-plane
go run main.go
# Wait for: "🚀 Control Plane listening on port 8081"
```

**Terminal 3 - Web UI:**
```bash
cd apps/web
bun dev
# Wait for: "▲ Next.js 14.1.0 - Local: http://localhost:3000"
```

### Step 3: Open Dashboard

Visit: **http://localhost:3000**

---

## 🎯 Quick Demo Actions

### 1. Ingest Events
Click **"Ingest Demo Event"** button → Watch stats update in real-time

### 2. Query by Entity
1. Copy the entity ID shown in the button (e.g., `user-789`)
2. Paste into "Entity ID" filter
3. Click "Search"
4. Expand an event to see the payload

### 3. API Test
```bash
# Ingest via API
curl -X POST http://localhost:8080/api/v1/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "test.event",
    "entity_id": "demo-123",
    "payload": {"message": "Hello AllSource!"}
  }'

# Query the event
curl "http://localhost:8080/api/v1/events/query?entity_id=demo-123" | jq
```

---

## 🐛 Troubleshooting

### "Port already in use"
```bash
# Check what's using the ports
lsof -i :8080  # Core
lsof -i :8081  # Control Plane
lsof -i :3000  # Web UI

# Kill if needed
kill -9 <PID>
```

### "Rust/Go/Bun not found"
- Install Rust: https://rustup.rs/
- Install Go: https://go.dev/dl/
- Install Bun: https://bun.sh/

### "Service won't start"
```bash
# Check logs for errors
cd services/core && cargo run  # See Rust errors
cd services/control-plane && go run main.go  # See Go errors
cd apps/web && bun dev  # See Next.js errors
```

### "Can't connect to Core from Control Plane"
Make sure Core is running first! Control Plane depends on Core.

---

## 📝 Useful Commands

```bash
# View all make targets
make help

# Install everything
make install

# Run all services (requires tmux or manually open 3 terminals)
make dev

# Clean all builds
make clean

# Run demo script (after services are running)
./demo-script.sh

# Format code
bun format
```

---

## 🔗 Service URLs

| Service | URL | Purpose |
|---------|-----|---------|
| Web UI | http://localhost:3000 | Visual dashboard |
| Event Store API | http://localhost:8080 | Core event operations |
| Control Plane | http://localhost:8081 | Cluster management |

---

## 📚 Next Steps

1. **Read the full README:** `README.md`
2. **Understand the architecture:** `ARCHITECTURE.md`
3. **Prepare for demo:** `DEMO.md`
4. **Try the demo script:** `./demo-script.sh`

---

## 🆘 Still Stuck?

Check these files:
- `README.md` - Full documentation
- `DEMO.md` - Presentation guide
- `ARCHITECTURE.md` - Technical deep-dive

Or create an issue describing your problem!

---

<div align="center">

**AllSource** - *You're ready to roll!* 🚀

</div>
