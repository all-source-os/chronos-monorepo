# Tenant Management Architecture (Without PostgreSQL)

**Date**: November 4, 2025
**Status**: ✅ CURRENT
**Question**: How do we support tenants without PostgreSQL?

---

## TL;DR Answer

**Both Core and Control-Plane already use in-memory storage for tenants.**
**No PostgreSQL is currently used anywhere for tenant management.**

---

## Current Reality

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│ Rust Core (Port 3900)                           │
│ ┌─────────────────────────────────────────────┐ │
│ │ TenantManager (in-memory)                   │ │
│ │ • DashMap<String, Tenant>                   │ │
│ │ • Create, get, update, delete               │ │
│ │ • Default tenant auto-created               │ │
│ │ • No PostgreSQL used                        │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
           │
           │ HTTP API (tenant validation)
           ▼
┌─────────────────────────────────────────────────┐
│ Go Control-Plane (Port 3901)                    │
│ ┌─────────────────────────────────────────────┐ │
│ │ MemoryTenantRepository (in-memory)          │ │
│ │ • map[string]*Tenant with sync.RWMutex      │ │
│ │ • Save, FindByID, FindAll, Update, Delete   │ │
│ │ • Clean Architecture implementation         │ │
│ │ • No PostgreSQL used                        │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**Total PostgreSQL instances for tenants: 0** ✅

---

## Current Implementation Details

### 1. Rust Core - TenantManager

**File**: `apps/core/src/tenant.rs` (Line 281)

**Storage**:
```rust
pub struct TenantManager {
    tenants: Arc<DashMap<String, Tenant>>,
}
```

**Features**:
- In-memory DashMap (lock-free, concurrent)
- Default tenant auto-created on startup
- Fast lookups (11.9 μs average)
- Thread-safe with atomic operations

**Methods**:
```rust
impl TenantManager {
    pub fn new() -> Self                                    // Creates with default tenant
    pub fn create_tenant(&self, id, name, quotas) -> Result<Tenant>
    pub fn get_tenant(&self, tenant_id: &str) -> Result<Tenant>
    pub fn update_usage(&self, tenant_id: &str, usage: &TenantUsage)
    pub fn check_quota(&self, tenant_id: &str) -> Result<bool>
    pub fn list_tenants(&self) -> Vec<Tenant>
}
```

**Initialization** (`apps/core/src/main.rs`):
```rust
let tenant_manager = Arc::new(TenantManager::new());
// Default tenant "default" is auto-created
```

---

### 2. Go Control-Plane - MemoryTenantRepository

**File**: `apps/control-plane/internal/infrastructure/persistence/memory_tenant_repository.go`

**Storage**:
```go
type MemoryTenantRepository struct {
    tenants map[string]*entities.Tenant
    mu      sync.RWMutex
}
```

**Features**:
- In-memory map with RWMutex for concurrency
- Clean Architecture repository pattern
- Full CRUD operations
- Active/inactive tenant filtering

**Methods**:
```go
func NewMemoryTenantRepository() *MemoryTenantRepository
func (r *MemoryTenantRepository) Save(tenant *Tenant) error
func (r *MemoryTenantRepository) FindByID(id string) (*Tenant, error)
func (r *MemoryTenantRepository) FindAll() ([]*Tenant, error)
func (r *MemoryTenantRepository) FindActive() ([]*Tenant, error)
func (r *MemoryTenantRepository) Update(tenant *Tenant) error
func (r *MemoryTenantRepository) Delete(id string) error
func (r *MemoryTenantRepository) Exists(id string) (bool, error)
```

**Initialization** (`apps/control-plane/internal/container.go`):
```go
tenantRepo := persistence.NewMemoryTenantRepository()
createTenantUC := usecases.NewCreateTenantUseCase(tenantRepo, auditRepo)
```

---

## Why In-Memory Works

### For Development/Small-Scale

**Advantages**:
- ✅ Extremely fast (11.9 μs lookups)
- ✅ Zero external dependencies
- ✅ Simple deployment
- ✅ No database setup/maintenance
- ✅ Atomic operations (thread-safe)

**Limitations**:
- ⚠️ Data lost on restart
- ⚠️ Single-node only (no distribution)
- ⚠️ Limited to RAM size
- ⚠️ No persistent audit trail

**Suitable For**:
- Development environments
- Testing
- Demo/POC deployments
- Small-scale production (<100 tenants)
- Ephemeral tenants (can recreate on restart)

---

### When You Need Persistence

If you need tenant data to survive restarts, you have **three options**:

---

## Option 1: Use Core's Existing PostgreSQL Feature (Recommended)

**Status**: Code exists, not currently used

**File**: `apps/core/src/infrastructure/repositories/postgres_tenant_repository.rs`

**Enable**:
```bash
cd apps/core

# Build with PostgreSQL feature
cargo build --features postgres

# Update main.rs to use PostgreSQL repository
```

**Benefits**:
- ✅ Already implemented (443 lines of code)
- ✅ PostgreSQL support for Core tenants
- ✅ ACID guarantees
- ✅ Persistent storage
- ✅ No code changes needed

**What You Get**:
```rust
#[cfg(feature = "postgres")]
pub struct PostgresTenantRepository {
    pool: PgPool,
}

// All methods implemented:
// - save, find_by_id, find_all, update, delete
// - Complex queries (find_by_status, find_active)
// - Usage tracking
// - Migrations included
```

**Setup**:
```bash
# 1. Start PostgreSQL
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:15

# 2. Update main.rs
let pool = PgPool::connect("postgresql://postgres:postgres@localhost/allsource").await?;
let tenant_repo = PostgresTenantRepository::new(pool.clone());
let tenant_manager = TenantManager::with_repository(tenant_repo);

# 3. Build & run
cargo build --features postgres
cargo run
```

**Migration** (`apps/core/migrations/001_tenants.sql`):
```sql
CREATE TABLE tenants (
    id VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    quotas JSONB NOT NULL,
    usage JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_status ON tenants(status);
CREATE INDEX idx_tenants_created ON tenants(created_at);
```

**Result**: Single PostgreSQL instance for both Core and Control-Plane

---

## Option 2: Sync to Core's DashMap API (Lightweight)

**Status**: Would need implementation

**Concept**: Control-Plane syncs tenants to Core's in-memory storage

**Architecture**:
```
┌────────────────────────────────────┐
│ Control-Plane (Port 3901)          │
│ • Creates tenant via API           │
│ • In-memory cache (fast reads)     │
└────────┬───────────────────────────┘
         │
         │ POST /api/v1/tenants
         ▼
┌────────────────────────────────────┐
│ Core (Port 3900)                   │
│ • DashMap storage (source of truth)│
│ • Optional Parquet persistence     │
└────────────────────────────────────┘
```

**Implementation**:

**Step 1: Add Core API endpoint** (Rust)
```rust
// apps/core/src/api_v1/tenants.rs

#[derive(Deserialize)]
pub struct CreateTenantRequest {
    pub id: String,
    pub name: String,
    pub quotas: TenantQuotas,
}

pub async fn create_tenant(
    State(tenant_manager): State<Arc<TenantManager>>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<Tenant>> {
    let tenant = tenant_manager.create_tenant(req.id, req.name, req.quotas)?;
    Ok(Json(tenant))
}

pub async fn get_tenant(
    State(tenant_manager): State<Arc<TenantManager>>,
    Path(tenant_id): Path<String>,
) -> Result<Json<Tenant>> {
    let tenant = tenant_manager.get_tenant(&tenant_id)?;
    Ok(Json(tenant))
}
```

**Step 2: Control-Plane syncs to Core** (Go)
```go
// apps/control-plane/internal/infrastructure/persistence/core_sync_tenant_repository.go

type CoreSyncTenantRepository struct {
    coreURL string
    cache   map[string]*entities.Tenant
    mu      sync.RWMutex
}

func (r *CoreSyncTenantRepository) Save(tenant *entities.Tenant) error {
    // Save to Core via HTTP
    resp, err := http.Post(
        r.coreURL+"/api/v1/tenants",
        "application/json",
        marshalTenant(tenant),
    )
    if err != nil {
        return err
    }

    // Cache locally
    r.mu.Lock()
    r.cache[tenant.ID] = tenant
    r.mu.Unlock()

    return nil
}

func (r *CoreSyncTenantRepository) FindByID(id string) (*entities.Tenant, error) {
    // Check cache first
    r.mu.RLock()
    if cached, ok := r.cache[id]; ok {
        r.mu.RUnlock()
        return cached, nil
    }
    r.mu.RUnlock()

    // Fetch from Core
    resp, err := http.Get(r.coreURL + "/api/v1/tenants/" + id)
    if err != nil {
        return nil, err
    }

    var tenant entities.Tenant
    json.NewDecoder(resp.Body).Decode(&tenant)

    // Update cache
    r.mu.Lock()
    r.cache[id] = &tenant
    r.mu.Unlock()

    return &tenant, nil
}
```

**Step 3: Wire up in container** (Go)
```go
// apps/control-plane/internal/container.go

func NewContainer() *Container {
    coreURL := os.Getenv("CORE_URL") // "http://localhost:3900"

    // Use Core sync repository instead of in-memory
    tenantRepo := persistence.NewCoreSyncTenantRepository(coreURL)

    // Rest stays the same
    createTenantUC := usecases.NewCreateTenantUseCase(tenantRepo, auditRepo)
    // ...
}
```

**Benefits**:
- ✅ Single source of truth (Core's DashMap)
- ✅ Fast local cache (Control-Plane)
- ✅ Optional Core Parquet persistence
- ✅ No PostgreSQL needed
- ✅ Consistent with projection state pattern

**Effort**: ~1 week implementation

---

## Option 3: Add PostgreSQL to Control-Plane (Not Recommended)

**Status**: Would need implementation

**Why Not Recommended**:
- ❌ Adds separate database instance
- ❌ Operational complexity
- ❌ Slower than in-memory (1-5ms vs 11.9 μs)
- ❌ Data duplication (Core also has tenants)
- ❌ Sync issues between Core and Control-Plane

**Only if**: You absolutely need Control-Plane independence from Core

---

## Recommended Approach

### For Development/Demo (Current) ✅
**Use**: In-memory (current implementation)
**Why**: Fast, simple, zero dependencies
**Trade-off**: Data lost on restart (acceptable for dev)

### For Production (Small Scale)
**Use**: Option 2 - Sync to Core's DashMap API
**Why**:
- Leverage existing Core infrastructure
- Fast (11.9 μs reads)
- Optional Parquet persistence
- Zero PostgreSQL needed
**Effort**: ~1 week

### For Production (Large Scale/Multi-Region)
**Use**: Option 1 - Enable Core's PostgreSQL feature
**Why**:
- Already implemented
- ACID guarantees
- Complex queries
- Multi-region replication (PostgreSQL feature)
**Effort**: ~1 day (enable feature + config)

---

## Migration Path

### Current → Production (Recommended)

**Phase 1: Enable Core PostgreSQL** (Day 1)
```bash
# 1. Start PostgreSQL
docker run -d postgres:15

# 2. Enable in Core
cargo build --features postgres

# 3. Update main.rs
let tenant_manager = TenantManager::with_postgres(pool);
```

**Phase 2: Control-Plane Sync to Core** (Week 1)
```bash
# 1. Add Core tenant API endpoints
# 2. Implement CoreSyncTenantRepository in Go
# 3. Update container.go to use sync repository
```

**Result**:
- ✅ Core tenants in PostgreSQL (ACID, persistent)
- ✅ Control-Plane caches from Core (fast reads)
- ✅ Single PostgreSQL instance (shared)
- ✅ No data duplication

---

## Architecture Decision

### Current State
```
Core: DashMap (in-memory, fast)
Control-Plane: map (in-memory, fast)
Persistence: None
Sync: None
```

### Recommended Production State
```
Core: PostgreSQL (persistent) + DashMap (cache)
Control-Plane: Sync to Core via API + local cache
Persistence: Core's PostgreSQL (single instance)
Sync: Control-Plane → Core HTTP API
```

**PostgreSQL instances needed: 1 (in Core, optional)**

---

## Code Example: Complete Production Setup

### Step 1: Core with PostgreSQL

```rust
// apps/core/src/main.rs

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to PostgreSQL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/allsource".to_string());

    #[cfg(feature = "postgres")]
    let pool = {
        use sqlx::PgPool;
        PgPool::connect(&database_url).await?
    };

    // Initialize with PostgreSQL
    #[cfg(feature = "postgres")]
    let tenant_manager = {
        use crate::infrastructure::repositories::PostgresTenantRepository;
        let repo = PostgresTenantRepository::new(pool.clone());
        repo.migrate().await?; // Run migrations
        Arc::new(TenantManager::with_repository(repo))
    };

    // Fallback to in-memory for non-postgres builds
    #[cfg(not(feature = "postgres"))]
    let tenant_manager = Arc::new(TenantManager::new());

    // ... rest of setup
}
```

### Step 2: Control-Plane Sync

```go
// apps/control-plane/internal/container.go

func NewContainer() *Container {
    coreURL := getEnv("CORE_URL", "http://localhost:3900")
    usePersistence := getEnv("USE_PERSISTENCE", "false") == "true"

    var tenantRepo repositories.TenantRepository

    if usePersistence {
        // Production: Sync to Core's PostgreSQL
        tenantRepo = persistence.NewCoreSyncTenantRepository(coreURL)
    } else {
        // Development: In-memory only
        tenantRepo = persistence.NewMemoryTenantRepository()
    }

    // ... rest stays same
}
```

---

## Summary

### Question: How do we support tenants without PostgreSQL?

**Answer**: We already do!

- ✅ **Current**: Both Core and Control-Plane use in-memory storage
- ✅ **Production Option 1**: Enable Core's PostgreSQL feature (1 day)
- ✅ **Production Option 2**: Sync to Core's DashMap API (1 week)
- ✅ **Either way**: Zero or one PostgreSQL instance total

**No additional database infrastructure needed beyond what exists.**

---

**Document Status**: ✅ CURRENT
**Version**: 1.0
**Last Updated**: November 4, 2025
