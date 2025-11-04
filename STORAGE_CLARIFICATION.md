# Storage Architecture Clarification

**Date**: November 4, 2025
**Critical Finding**: PostgreSQL is NOT currently used

---

## Current Reality

### What's ACTUALLY Running (Default Build)

```rust
// apps/core/src/main.rs - Line 28
let store = Arc::new(EventStore::new());
```

**Storage Stack:**
```
┌─────────────────────────────────────────┐
│ CURRENT PRODUCTION STORAGE              │
├─────────────────────────────────────────┤
│ 1. In-Memory (Primary)                 │
│    • DashMap for indexing              │
│    • Vec<Event> in RwLock              │
│    • 11.9 μs query latency             │
│                                         │
│ 2. Parquet (Optional Persistence)      │
│    • Columnar storage                  │
│    • Append-only files                 │
│    • Enabled via storage_dir config    │
│                                         │
│ 3. WAL (Write-Ahead Log)               │
│    • Durability & crash recovery       │
│    • Always enabled                    │
│    • CRC32 checksums                   │
└─────────────────────────────────────────┘
```

### What's NOT Running (But Available)

```toml
# apps/core/Cargo.toml - Line 84
sqlx = { version = "0.7", features = [...], optional = true }

# Line 89
[features]
default = []
postgres = ["sqlx"]  # NOT IN DEFAULT
```

**PostgreSQL Implementation EXISTS but is:**
- ✅ Fully implemented (3 repositories)
- ❌ **NOT compiled in default build**
- ❌ **NOT enabled in main.rs**
- ❌ **Feature-gated behind `--features postgres`**

**Available Repositories (NOT USED):**
1. `PostgresTenantRepository` - Tenant management
2. `PostgresEventStreamRepository` - Event storage
3. `PostgresAuditRepository` - Audit logs

---

## Why PostgreSQL Exists

### It's an OPTION, not a requirement:

```bash
# Default build (what's actually running)
cargo build
# Uses: In-memory + Parquet + WAL

# Optional PostgreSQL build (not used)
cargo build --features postgres
# Would use: PostgreSQL + In-memory + Parquet + WAL
```

### Use Cases for PostgreSQL Feature:
1. **ACID Transactions**: When you need strong consistency
2. **Complex Queries**: SQL joins, aggregations
3. **Multi-region**: PostgreSQL replication
4. **Compliance**: Some industries require relational DB

---

## Revised Understanding

### ❌ INCORRECT Statement (from our analysis)
> "Keep 1 PostgreSQL instance for Core"

### ✅ CORRECT Statement
> "Core currently uses NO PostgreSQL. PostgreSQL is an optional feature available if needed for specific use cases."

---

## Current Storage Strategy

### Development/Testing (Current)
```
In-Memory (DashMap + Vec)
  ↓ (optional)
Parquet files (for persistence)
  ↓ (always)
WAL (for durability)
```

**Benefits:**
- ✅ Extremely fast (11.9 μs queries)
- ✅ Simple deployment (no external DB)
- ✅ Parquet for analytics
- ✅ WAL for crash recovery

**Limitations:**
- ⚠️ Data fits in RAM
- ⚠️ Single-node only
- ⚠️ No SQL queries
- ⚠️ Manual backup/restore

---

### Production Options (If Needed)

#### Option 1: Current + Parquet (Recommended for most cases)
```
In-Memory (hot data)
  ↓
Parquet (persistent cold data)
  ↓
WAL (durability)
```

**Use when:**
- High throughput needed (469K events/sec)
- Data fits in memory (or hot/cold split)
- Don't need SQL
- Analytics are important

---

#### Option 2: Add PostgreSQL (For specific needs)
```bash
# Build with PostgreSQL
cargo build --features postgres

# Update main.rs to use PostgresTenantRepository
```

```
PostgreSQL (ACID, SQL queries)
  +
In-Memory (fast cache)
  +
Parquet (analytics)
  +
WAL (durability)
```

**Use when:**
- Need ACID transactions
- Need SQL joins/aggregations
- Multi-region deployment
- Compliance requires RDBMS

---

#### Option 3: RocksDB (High-performance embedded)
```bash
cargo build --features rocksdb-storage
```

```
RocksDB (embedded key-value)
  +
In-Memory (cache)
  +
Parquet (analytics)
```

**Use when:**
- Need persistence
- Don't want external DB
- Need better than in-memory capacity
- Still want high performance

---

## Recommendation: ZERO PostgreSQL Instances

### For Query-Service
❌ **Original Plan**: Add PostgreSQL for projection storage
✅ **Optimized Plan**: Use Core's in-memory DashMap (11.9 μs)

**Why:**
- Core's DashMap is 50-100x faster than PostgreSQL
- Projection state is ephemeral (can rebuild from events)
- If persistence needed, use Core's Parquet storage
- No operational overhead

---

### For Core
❌ **Assumption**: Need PostgreSQL for event storage
✅ **Reality**: Already using Parquet + WAL + In-Memory

**Why:**
- Current storage handles 469K events/sec
- Parquet is better for event sourcing (columnar, immutable)
- PostgreSQL is optional for specific use cases only
- Don't add complexity without need

---

## Updated Architecture Diagram

```
┌──────────────────────────────────────────────────────────┐
│                  CLIENT APPLICATIONS                      │
└─────────────────────┬────────────────────────────────────┘
                      │
                      ▼
┌──────────────────────────────────────────────────────────┐
│         Rust Core (Port 3900) - EVENT STORE              │
│  ┌────────────────────────────────────────────────────┐ │
│  │ STORAGE STACK (NO POSTGRESQL)                      │ │
│  ├────────────────────────────────────────────────────┤ │
│  │ L1: In-Memory DashMap                              │ │
│  │     • 11.9 μs queries                              │ │
│  │     • Lock-free concurrent access                  │ │
│  │     • Entity + Type indexes                        │ │
│  ├────────────────────────────────────────────────────┤ │
│  │ L2: Parquet Files (Optional)                       │ │
│  │     • Columnar storage                             │ │
│  │     • Snappy compression                           │ │
│  │     • Analytics queries                            │ │
│  ├────────────────────────────────────────────────────┤ │
│  │ L3: Write-Ahead Log                                │ │
│  │     • Durability                                   │ │
│  │     • Crash recovery                               │ │
│  │     • CRC32 checksums                              │ │
│  └────────────────────────────────────────────────────┘ │
│                                                          │
│  Optional (feature-gated, NOT USED):                    │
│  • PostgreSQL (--features postgres)                     │
│  • RocksDB (--features rocksdb-storage)                 │
└──────────────────────────────────────────────────────────┘
           │
           │ WebSocket + HTTP
           ▼
┌──────────────────────────────────────────────────────────┐
│      Elixir Query-Service (Port 3902)                    │
│  • Subscribe to Core's WebSocket                         │
│  • GenServer/ETS cache (local)                           │
│  • Sync state to Core's DashMap                          │
│  • NO PostgreSQL NEEDED                                  │
└──────────────────────────────────────────────────────────┘
```

---

## Final Answer to Your Question

### "What do we keep 1 PostgreSQL instance for?"

**Answer: NOTHING.**

We don't need PostgreSQL at all for the current architecture:

1. **Core Event Store**: Uses in-memory + Parquet + WAL (NO PostgreSQL)
2. **Query-Service Projections**: Use Core's DashMap API (NO PostgreSQL)
3. **Control-Plane**: Uses in-memory for demo/dev (NO PostgreSQL)

### PostgreSQL is only needed IF:
- You want ACID transactions across events
- You need complex SQL queries
- You need multi-region replication via PostgreSQL
- Compliance requires a relational database

**For 99% of use cases, the current storage (in-memory + Parquet + WAL) is superior.**

---

## Revised Operational Complexity

### Original Assessment (INCORRECT)
```
PostgreSQL instances: 2 → 1 (50% reduction)
```

### Corrected Assessment
```
PostgreSQL instances: 0 → 0 (NO CHANGE, NONE NEEDED)
```

### Actual Storage Infrastructure
```
Current:
- 0 PostgreSQL instances ✅
- 0 Redis instances ✅
- 1 WebSocket server (Core) ✅
- Parquet files (local disk) ✅
- WAL files (local disk) ✅

No external dependencies needed! 🎉
```

---

## Performance Reality Check

| Storage | Latency | Throughput | Use Case |
|---------|---------|------------|----------|
| **DashMap (in-memory)** | **11.9 μs** | **469K events/sec** | **Current, perfect for event store** |
| PostgreSQL | 1-5 ms | ~10K events/sec | ACID, SQL queries |
| Redis | 0.5-1 ms | 100K ops/sec | Cache only |
| Parquet | ~5-10 ms | N/A | Analytics, cold storage |

**Current choice (DashMap + Parquet + WAL) is optimal for event sourcing.**

---

## Updated Recommendations

### ✅ KEEP Current Storage
- In-memory DashMap (11.9 μs)
- Optional Parquet persistence
- WAL for durability

### ❌ DO NOT ADD
- PostgreSQL to Query-service (not needed)
- Redis to Query-service (slower than DashMap)
- PostgreSQL to Core (unless specific need)

### 📋 OPTIONAL (Future, if needed)
- Enable `--features postgres` if you need:
  - ACID transactions
  - SQL queries
  - Multi-region PostgreSQL replication
  - RDBMS for compliance

---

## Conclusion

**We don't need ANY PostgreSQL instances for the current architecture.**

The optimization plan should be updated to reflect:
- **Current PostgreSQL instances**: 0
- **Planned PostgreSQL instances**: 0
- **Storage**: In-memory (DashMap) + Parquet + WAL

This is actually **better** than we thought:
- ✅ Zero external database dependencies
- ✅ 11.9 μs query latency
- ✅ 469K events/sec throughput
- ✅ Parquet for analytics
- ✅ WAL for durability
- ✅ Simple deployment (no DB to manage)

**The architecture is simpler and faster than initially assessed!**

---

**Status**: Critical Clarification Complete
**Impact**: Architecture is even simpler than planned
**Action**: Update all documents to reflect zero PostgreSQL dependency
