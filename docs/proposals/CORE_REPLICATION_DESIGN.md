# Core Replication Design

> **Status**: Proposal
> **Author**: Design session 2026-02-12
> **Scope**: AllSource Core — high availability, read scaling, and failover

---

## 1. Current State

### What We Have (Single-Node)

AllSource Core is a Rust event store with a three-tier storage stack and zero external database dependencies:

```
                    +-----------------------+
                    |   AllSource Core       |
                    |   (single instance)    |
                    +-----------+-----------+
                                |
            +-------------------+-------------------+
            |                   |                   |
   +--------v--------+ +-------v--------+ +--------v--------+
   | L1: DashMap      | | L2: Parquet    | | L3: WAL         |
   | (in-memory)      | | (columnar)     | | (append-only)   |
   | 11.9us queries   | | Snappy codec   | | CRC32 checksums |
   | 469K events/sec  | | 5-min flush    | | 100ms fsync     |
   +------------------+ +----------------+ +-----------------+
```

**Write path**: Event -> DashMap -> WAL (sync) -> Parquet (async batch)
**Read path**: DashMap lookup -> return (all reads served from memory)
**Durability**: WAL crash recovery + Parquet snapshots. No replication.

### What We Have in the Broader Stack

```
  Browser / API Client
         |
         v
  +------------------+          +------------------+
  | Query Service    |  HTTP    | Core             |
  | (Elixir/Phoenix) |-------->| (Rust/Axum)      |
  | Port 3902        |  WS     | Port 3900        |
  |                  |<--------|                   |
  | Auth, billing,   |         | Events, queries,  |
  | rate limiting    |         | projections,      |
  | tenant isolation |         | snapshots, WAL    |
  +--------+---------+         +------------------+
           |
  +--------v---------+
  | PostgreSQL       |
  | (metadata only)  |
  | tenants, api_keys|
  | usage, billing   |
  +------------------+
```

### Single Points of Failure

| Component | Failure Mode | Impact |
|-----------|-------------|--------|
| Core | Process crash | WAL recovery on restart. Brief downtime. |
| Core | Node dies | **All data in DashMap lost.** WAL recovery from disk. |
| Core | Disk failure | **Data loss.** No offsite copy. |
| Core | Overloaded | All reads AND writes degrade together. |

---

## 2. Design Goals

1. **No data loss** — a single node failure must not lose committed events
2. **Read availability** — reads continue during a write-node failure
3. **Horizontal read scaling** — add read replicas to handle query load
4. **Minimal write-path overhead** — replication must not significantly degrade ingestion throughput
5. **Operational simplicity** — no Raft, no consensus quorum, no split-brain complexity in Phase 1
6. **Core IS the database** — no external databases in the event data path

---

## 3. Architecture: WAL-Based Replication

### Core Idea

The WAL already captures every event in order. Ship it to followers. Followers replay into their own DashMap. Reads can go to any node. Writes go to the leader.

This is the same principle as PostgreSQL streaming replication, MySQL binlog replication, and Kafka ISR — proven at scale.

### C4 Context Diagram

```
+-------------------------------------------------------------------+
|                        System Context                              |
+-------------------------------------------------------------------+

  +-------------+     +-------------+     +------------------+
  | Web App     |     | API Clients |     | MCP (AI Agents)  |
  | (Next.js)   |     | (SDKs)      |     | (Claude, etc.)   |
  +------+------+     +------+------+     +--------+---------+
         |                   |                      |
         +-------------------+----------------------+
                             |
                             v
                  +----------+----------+
                  |   Query Service     |
                  |   (Elixir/Phoenix)  |
                  |                     |
                  |  Auth, rate limits, |
                  |  routing, billing   |
                  +----+----------+-----+
                       |          |
              writes   |          |  reads
                       v          v
              +--------+--+  +---+----------+
              | Core       |  | Core         |
              | LEADER     |  | FOLLOWER(s)  |
              | (read+write|  | (read-only)  |
              +--------+---+  +---+----------+
                       |          ^
                       |   WAL    |
                       +----------+
                       replication
```

### C4 Container Diagram

```
+-----------------------------------------------------------------------+
|                        Container Diagram                               |
+-----------------------------------------------------------------------+

  +-------------------+
  | Query Service     |
  | [Elixir/Phoenix]  |
  |                   |
  | Responsibilities: |
  | - Auth (JWT/OAuth)|
  | - Tenant isolation|
  | - Rate limiting   |
  | - Write routing   |
  | - Read balancing  |
  | - Usage metering  |
  +----+--------+-----+
       |        |
       | POST   | GET
       | events | events/query
       v        v
  +----+----+  +----------+  +----------+
  | Core    |  | Core     |  | Core     |
  | LEADER  |  | FOLLOWER |  | FOLLOWER |
  | [Rust]  |  | [Rust]   |  | [Rust]   |
  |         |  |          |  |          |
  | DashMap |  | DashMap  |  | DashMap  |
  | WAL     |  | WAL      |  | WAL      |
  | Parquet |  | Parquet  |  | Parquet  |
  +---------+  +----------+  +----------+
       |            ^              ^
       |  WAL ship  |   WAL ship   |
       +------------+--------------+
```

### C4 Component Diagram (Core Internals — Post-Replication)

```
+-----------------------------------------------------------------------+
|                    Core Component Diagram                               |
+-----------------------------------------------------------------------+

  +------------------------------------------------------+
  | AllSource Core (Leader or Follower)                   |
  |                                                       |
  |  +------------------+    +------------------------+   |
  |  | HTTP/WS Server   |    | Replication Module     |   |
  |  | (Axum)           |    |                        |   |
  |  | - /api/v1/events |    | Leader:                |   |
  |  | - /api/v1/query  |    |  - WAL Shipper         |   |
  |  | - /health        |    |  - Follower Registry   |   |
  |  | - /ws stream     |    |  - Ack Tracker         |   |
  |  +--------+---------+    |                        |   |
  |           |               | Follower:              |   |
  |           v               |  - WAL Receiver        |   |
  |  +--------+---------+    |  - Replay Engine        |   |
  |  | Event Ingestion  |    |  - Catch-up Logic       |   |
  |  | Use Case         |    +----------+-------------+   |
  |  +--------+---------+               |                 |
  |           |                          |                 |
  |           v                          v                 |
  |  +--------+----------------------------------+        |
  |  | Storage Layer                              |        |
  |  |                                            |        |
  |  |  +----------+  +--------+  +-----------+   |        |
  |  |  | DashMap   |  | WAL    |  | Parquet   |   |        |
  |  |  | (memory)  |  | (disk) |  | (disk)    |   |        |
  |  |  +----------+  +--------+  +-----------+   |        |
  |  +--------------------------------------------+        |
  +--------------------------------------------------------+
```

---

## 4. Replication Protocol

### WAL Shipping (Leader -> Follower)

The leader already writes every event to the WAL (JSON-line format with CRC32 checksums). Replication extends this:

```
Leader                              Follower
  |                                    |
  |  1. Event ingested                 |
  |  2. Written to DashMap             |
  |  3. Written to WAL                 |
  |  4. WAL entry shipped ------------>|
  |                                    | 5. Validate CRC32
  |                                    | 6. Write to local WAL
  |                                    | 7. Replay into DashMap
  |  8. Ack received <----------------|
  |                                    |
```

### WAL Entry Format (Existing)

```json
{"event_id":"uuid","entity_id":"user-123","event_type":"user.created","tenant_id":"t1","payload":{...},"timestamp":"...","version":1,"crc32":"a1b2c3d4"}
```

No format changes needed. The WAL is already the replication log.

### Shipping Mechanism

**Option A: Push via persistent TCP connection (recommended)**
- Leader maintains a TCP stream to each follower
- Streams WAL entries as they're written
- Follower sends ACK with last-applied WAL offset
- Leader tracks each follower's position

**Option B: Pull via HTTP long-poll**
- Follower polls `GET /internal/wal?after=<offset>`
- Simpler but higher latency
- Better for cross-datacenter with unreliable networks

Recommend **Option A** for same-datacenter deployments, **Option B** as fallback.

### Catch-Up Protocol

When a follower falls behind or starts fresh:

```
Follower                              Leader
  |                                      |
  | 1. "I need events after offset X" -->|
  |                                      | 2. Check: is offset X in WAL?
  |                                      |
  |    [If yes: stream from WAL]         |
  |  <-- WAL entries from offset X ------|
  |                                      |
  |    [If no: WAL rotated past X]       |
  |  <-- Full Parquet snapshot --------- |
  |  <-- Then WAL from snapshot point ---|
  |                                      |
```

This mirrors PostgreSQL's pg_basebackup + streaming replication pattern.

---

## 5. Deployment Topology

### Minimum Production (3 nodes)

```
  +-------------------+
  | Query Service     |
  | (routing layer)   |
  +----+--------+-----+
       |        |
  writes|    reads (round-robin)
       |        |
       v        +------------------+
  +----+----+   |                  |
  | Core    |   |   +-----------+  |   +-----------+
  | LEADER  +-->+-->| FOLLOWER  |  +-->| FOLLOWER  |
  | node-1  |  WAL  | node-2    |  WAL | node-3    |
  +---------+       +-----------+      +-----------+
```

**Leader** (1 instance):
- Accepts all writes (`POST /api/v1/events`)
- Serves reads (can offload to followers)
- Ships WAL entries to all followers
- Runs Parquet compaction

**Followers** (N instances):
- Read-only (reject writes with `409 Conflict` or redirect)
- Receive and replay WAL stream
- Serve read queries independently
- Maintain own DashMap, WAL, and Parquet files

### Docker Compose (Target)

```yaml
services:
  core-leader:
    image: ghcr.io/all-source-os/chronos-core:1.0.0
    environment:
      ALLSOURCE_ROLE: leader
      ALLSOURCE_REPLICATION_ENABLED: "true"
      ALLSOURCE_REPLICATION_PORT: 3910
    ports:
      - "3280:3900"   # HTTP API
      - "3910:3910"   # Replication port
    volumes:
      - core_leader_data:/app/data

  core-follower-1:
    image: ghcr.io/all-source-os/chronos-core:1.0.0
    environment:
      ALLSOURCE_ROLE: follower
      ALLSOURCE_LEADER_URL: "core-leader:3910"
      ALLSOURCE_READ_ONLY: "true"
    ports:
      - "3281:3900"
    volumes:
      - core_follower1_data:/app/data

  core-follower-2:
    image: ghcr.io/all-source-os/chronos-core:1.0.0
    environment:
      ALLSOURCE_ROLE: follower
      ALLSOURCE_LEADER_URL: "core-leader:3910"
      ALLSOURCE_READ_ONLY: "true"
    ports:
      - "3282:3900"
    volumes:
      - core_follower2_data:/app/data
```

---

## 6. Query Service as Router

The query service gains a new responsibility: routing writes to the leader and distributing reads across followers.

### Routing Logic

```
                       Query Service
                            |
                +-----------+-----------+
                |                       |
           Is it a write?          Is it a read?
           (POST /events)         (GET /events/query)
                |                       |
                v                       v
         Route to LEADER        Round-robin across
                                LEADER + FOLLOWERs
```

### Implementation in Query Service

New config:

```elixir
# config/runtime.exs
config :query_service_ex,
  core_write_url: System.get_env("CORE_WRITE_URL") || "http://localhost:3900",
  core_read_urls: System.get_env("CORE_READ_URLS") |> parse_comma_separated()
  # CORE_READ_URLS="http://core-leader:3900,http://core-follower-1:3900,http://core-follower-2:3900"
```

The RustCoreClient would use `core_write_url` for event creation and round-robin across `core_read_urls` for queries. If a read node is unhealthy, skip it.

---

## 7. Failover

### Phase 1: Manual Failover (Operational)

No automatic leader election. If the leader dies:

1. Operator verifies leader is down
2. Promote a follower: `ALLSOURCE_ROLE=leader` + restart
3. Point remaining followers to the new leader
4. Update query service `CORE_WRITE_URL`

This is the same model as Redis Sentinel's manual failover or PostgreSQL with pg_basebackup before Patroni.

**Acceptable because**: leader failure is rare (months between events), and recovery is minutes, not hours.

### Phase 2: Automatic Failover (Future)

When the operational cost of manual failover justifies the complexity:

```
  +-------------------+
  | Sentinel Process  |  (lightweight health checker)
  | (Rust or Elixir)  |
  +----+---------+----+
       |         |
  heartbeat  heartbeat
       |         |
  +----v----+  +-v----------+
  | LEADER  |  | FOLLOWER   |
  +---------+  +------------+

  Leader misses 3 heartbeats (30s):
  1. Sentinel promotes follower with highest WAL offset
  2. Sentinel updates query service config via API
  3. Other followers reconnect to new leader
```

No Raft needed. A single sentinel with a simple state machine is sufficient for 1 leader + N followers.

### Phase 3: Multi-Leader (Not Recommended Yet)

Multi-leader adds conflict resolution complexity (last-write-wins, vector clocks, CRDTs). The event store's append-only nature makes conflicts less likely but not impossible (duplicate entity_ids across leaders).

Defer until there's a proven need for geo-distributed writes.

---

## 8. Consistency Model

### Guarantee: Eventual Consistency for Reads

- Writes are committed when the leader's WAL is fsynced
- Followers are eventually consistent (lag = WAL shipping latency, typically < 100ms)
- A read immediately after a write may not see the new event on a follower

### Guarantee: Strong Consistency When Needed

For use cases requiring read-your-writes:

```
POST /api/v1/events  →  returns { event_id, timestamp }
GET /api/v1/events?after=<timestamp>  →  query with freshness hint
```

The query service can route "read-after-write" queries to the leader:

```elixir
# If request has X-Consistency: strong header, route to leader
def query_events(tenant_id, params, opts \\ []) do
  url = if opts[:consistency] == :strong, do: write_url(), else: next_read_url()
  Tesla.get(client(url), "/api/v1/events/query", query: params)
end
```

### Replication Lag Monitoring

Each follower exposes its lag:

```
GET /health
{
  "status": "healthy",
  "role": "follower",
  "replication_lag_ms": 45,
  "leader_wal_offset": 1000042,
  "follower_wal_offset": 1000038
}
```

The query service stops routing reads to a follower if `replication_lag_ms > threshold` (e.g., 5 seconds).

---

## 9. Data Safety

### Synchronous vs Asynchronous Replication

| Mode | Latency | Safety | Use Case |
|------|---------|--------|----------|
| **Async** (default) | ~0ms write overhead | Leader crash loses up to 100ms of events | Most deployments |
| **Semi-sync** | +1-5ms per write | At least 1 follower confirmed before ack | Financial/audit events |
| **Sync** | +5-20ms per write | All followers confirmed before ack | Regulatory compliance |

Default to **async**. Expose `ALLSOURCE_REPLICATION_MODE` env var.

For semi-sync, the leader waits for at least 1 follower ACK before returning 200 to the client:

```
Client -> Leader: POST event
Leader: write to DashMap + WAL
Leader: ship to followers
Leader: wait for 1 ACK (timeout 5s)
Leader -> Client: 200 OK (with event_id)
```

### Cross-Datacenter Backup

WAL shipping also enables offsite backup without a full replica:

```
Leader WAL ---> S3 / object storage (async, every N seconds)
```

Parquet files are already a natural backup format (self-contained, compressed, checksummed).

---

## 10. Implementation Phases

### Phase 1: Read Replicas (Weeks)

**Scope**: Ship WAL from leader to followers. Followers replay into DashMap. Query service routes reads.

**Changes to Core**:
- New env vars: `ALLSOURCE_ROLE`, `ALLSOURCE_LEADER_URL`, `ALLSOURCE_REPLICATION_PORT`
- Follower mode: reject writes, connect to leader's replication port
- Leader mode: accept follower connections, stream WAL entries
- Health endpoint: expose `role` and `replication_lag_ms`

**Changes to Query Service**:
- `CORE_WRITE_URL` / `CORE_READ_URLS` config
- RustCoreClient: separate write and read clients
- Read balancing with health-aware routing

**Changes to Docker Compose**:
- Leader + 2 follower containers
- Shared network
- Separate data volumes

### Phase 2: Semi-Sync + Monitoring (Weeks)

**Scope**: Semi-synchronous replication mode. Replication lag dashboard.

**Changes to Core**:
- `ALLSOURCE_REPLICATION_MODE=semi-sync`
- Leader waits for 1 follower ACK before responding
- Prometheus metrics: `allsource_replication_lag_seconds`, `allsource_replication_acks_total`

**Changes to Query Service**:
- Skip followers with high replication lag
- `X-Consistency: strong` header support

### Phase 3: Automated Failover (Months)

**Scope**: Sentinel process for automatic leader promotion.

**New Component**:
- Lightweight process (Rust or Elixir) that monitors leader health
- Promotes highest-offset follower on leader failure
- Notifies query service of new leader endpoint

---

## 11. What This Does NOT Include

- **Multi-leader / active-active**: Unnecessary complexity for append-only event stores
- **Raft consensus**: Overkill for 1 leader + N followers. Adds latency and operational burden.
- **Sharding / partitioning across nodes**: Core already has 256-partition infrastructure. Activate it only when single-node write throughput (469K events/sec) is insufficient.
- **PostgreSQL in the event path**: Never. Core is the database.

---

## 12. Decision Record

| Decision | Rationale |
|----------|-----------|
| WAL-based replication | WAL already exists, proven pattern (PG, MySQL, Kafka), minimal protocol overhead |
| Single leader, N followers | Simplicity. No conflict resolution. No consensus overhead. |
| Query service as router | Already in the data path. No new component needed. |
| Manual failover first | Leader failures are rare. Automate when operational cost justifies it. |
| Async replication default | 469K events/sec throughput preserved. Semi-sync opt-in for critical workloads. |
| No Raft | Raft solves leader election for stateful systems that need it. A simple sentinel + WAL offset comparison is sufficient here. |
| No PostgreSQL for events | Core's DashMap + WAL + Parquet stack is purpose-built. Adding PG adds latency, operational burden, and a dependency that provides no benefit over WAL shipping. |
