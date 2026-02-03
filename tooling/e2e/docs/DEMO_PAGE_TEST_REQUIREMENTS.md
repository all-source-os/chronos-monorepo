# Demo Page Structure and Test Requirements

This document describes the demo page architecture, UI elements, API dependencies, and test coverage requirements.

## 1. Route Structure

**Single Page Application with Feature Card Navigation**

| Aspect | Details |
|--------|---------|
| Route | `/demo` |
| Location | `apps/web/src/app/demo/page.tsx` |
| Architecture | Single-page with state-based tab switching (React hooks) |
| Navigation | 8 feature cards act as buttons to switch between demo sections |
| Multi-route | No - all sections render conditionally on the same page |

## 2. Demo Sections and UI Elements

### Section 1: Event Ingestion (`event-ingestion-demo.tsx`)

**Purpose**: Generate and ingest sample events in batches

**UI Elements**:
- Batch size selector (5, 10, 25, 50, 100 events)
- "Generate E-Commerce Events" button
- "Generate IoT Sensor Data" button
- Event stream visualization (last 5 batches)
- Stats cards: Total Batches, Total Events, Throughput, Success Rate
- Top Event Types breakdown with progress bars
- Expandable batch details showing first 3 events

**API Dependency**: Core API (with mock fallback)

---

### Section 2: Query Engine (`query-demo.tsx`)

**Purpose**: Execute queries on the event store

**Query Types**:
- By Entity
- By Type
- Time Range
- Advanced (combined filters)

**UI Elements**:
- Query parameter inputs (Entity ID, Event Type, Time Range, Result Limit)
- 4 query execution buttons
- Results table with expandable JSON details
- Query history sidebar (last 5 queries)
- Export results as JSON button
- "No results" fallback message

**API Dependency**: Core API (with mock fallback)

---

### Section 3: System Metrics (`metrics-demo.tsx`)

**Purpose**: Display real-time system performance metrics

**Metric Cards** (5 metrics):
1. Ingestion Rate (/s)
2. Query Latency (ms)
3. Active Connections
4. Storage Used (GB)
5. Total Events

**UI Elements**:
- Manual/Auto-refresh toggle button
- Refresh interval selector (5s, 10s, 30s, 60s)
- Mini trend charts (last 10 snapshots)
- Historical snapshots summary
- Last updated timestamp
- Trend indicators (up/down/stable)

**API Dependency**: Core API `/api/v1/metrics` (requires authentication)

---

### Section 4: Projections (`projections-demo.tsx`)

**Purpose**: Build materialized views from event streams

**Projection Types** (5):
1. Entity Snapshot - maintain current state
2. Event Counter - count by type/entity
3. Time Series - aggregate over time windows
4. Funnel Analysis - track multi-step flows
5. Custom Projection - user-defined logic

**UI Elements**:
- Projection type selector buttons
- Projection name input
- JSON configuration display
- Create/pause/resume controls
- Stats display (events processed, errors, avg time)
- Expandable projection details
- Status indicators (Running, Paused, Failed, Rebuilding)

**API Dependency**: Core API (with mock fallback)

---

### Section 5: Enterprise Security (`security-demo.tsx`)

**Purpose**: Showcase security features and tenant management

**Tabs**: Overview, Tenants, Audit Logs

**Security Features** (8 cards):
1. Multi-Tenancy
2. RBAC (Role-Based Access Control)
3. JWT Authentication
4. Rate Limiting
5. IP Filtering
6. Encryption
7. Audit Logs
8. API Keys

**UI Elements**:
- Security score progress bar
- Feature detail panels (JWT config, RBAC policy testing, rate limits)
- Tenant management section with creation capability
- Audit log viewer with sample entries
- Quota display per tenant
- Tenant tier badges (Free, Standard, Professional, Enterprise)

**API Dependency**: Control Plane API (with mock fallback)

---

### Section 6: Time Travel (`time-travel-demo.tsx`)

**Purpose**: Query historical state at any point in time

**UI Elements**:
- Entity ID input
- Timeline visualization with hourly points
- Playback controls (Play, Pause, Previous, Next, Reset, Fast Forward)
- Progress bar with percentage
- Entity state display (reconstructed from events)
- Events at selected time display
- Info box explaining time travel

**API Dependency**: Core API (with mock fallback)

---

### Section 7: Event Analytics (`analytics-demo.tsx`)

**Purpose**: Advanced analytics with multiple query types

**Analytics Options** (4):
1. Time Series Analysis (daily granularity)
2. Funnel Analysis (4-step conversion tracking)
3. Cohort Analysis (retention measurement)
4. Statistical Aggregations (11 functions)

**Aggregation Functions** (11):
count, sum, avg, min, max, stddev, variance, percentile, distinct, first, last

**UI Elements**:
- Analytics type selector cards
- Results display with trend indicators
- Mock data visualization (progress bars for distributions)
- Features showcase section
- Info banner with capabilities

**API Dependency**: Core API (with mock fallback)

---

### Section 8: Event Pipelines (`pipelines-demo.tsx`)

**Purpose**: Build composable event processing pipelines

**Operator Types** (10):
Filter, Map, Enrich, Window, Batch, Aggregate, Deduplicate, Throttle, Partition, Flat Map

**UI Elements**:
- Pipeline name input
- Operator buttons in 5-column grid
- Pipeline builder with operator sequence
- Operator removal and clear all controls
- Configuration JSON display per operator
- Create & Run Pipeline button
- Success message with pipeline details

**API Dependency**: Core API (with mock fallback)

---

## 3. API Dependencies

### Core Event Store API (Primary: `http://localhost:3900`)

| Endpoint | Method | Description | Used By |
|----------|--------|-------------|---------|
| `/api/events` | POST | Create single event | Event Ingestion |
| `/api/events/batch` | POST | Create multiple events | Event Ingestion |
| `/api/events` | GET | Query events (filters: entity_id, event_type, from_timestamp, to_timestamp, as_of, limit) | Query Engine, Time Travel |
| `/api/v1/metrics` | GET | Get system metrics | System Metrics |
| `/api/projections` | GET | List all projections | Projections |
| `/api/projections/{id}` | GET | Get projection details | Projections |
| `/api/projections` | POST | Create projection | Projections |
| `/api/projections/{id}/pause` | POST | Pause projection | Projections |
| `/api/projections/{id}/resume` | POST | Resume projection | Projections |
| `/api/analytics` | POST | Run analytics query | Event Analytics |
| `/api/pipelines` | GET | List pipelines | Event Pipelines |
| `/api/pipelines` | POST | Create pipeline | Event Pipelines |
| `/health` | GET | Health check | E2E Tests |
| `/api/events/stream` | WebSocket | Real-time event streaming | (Not used in demo) |

### Control Plane API (Secondary: `http://localhost:3901`)

| Endpoint | Method | Description | Used By |
|----------|--------|-------------|---------|
| `/api/v1/tenants` | GET | List tenants | Security |
| `/api/v1/tenants` | POST | Create tenant | Security |
| `/api/v1/policies/evaluate` | POST | Evaluate access policies | Security |
| `/health` | GET | Control plane health check | E2E Tests |

---

## 4. Expected API Response Shapes

### Event
```typescript
interface Event {
  id: string;
  event_type: string;
  entity_id: string;
  tenant_id: string;
  payload: Record<string, unknown>;
  metadata?: Record<string, unknown>;
  timestamp: string;
  version: number;
}
```

### Metrics
```typescript
interface Metrics {
  ingestion_rate: number;
  query_latency_ms: number;
  active_connections: number;
  storage_used_gb: number;
  events_total: number;
  timestamp: string;
}
```

### Projection
```typescript
interface Projection {
  id: string;
  name: string;
  projection_type: "EntitySnapshot" | "EventCounter" | "Custom" | "TimeSeries" | "Funnel";
  status: "Created" | "Running" | "Paused" | "Failed" | "Stopped" | "Rebuilding";
  config: ProjectionConfig;
  stats?: ProjectionStats;
  created_at: string;
  updated_at: string;
}
```

### Pipeline
```typescript
interface Pipeline {
  id: string;
  name: string;
  operators: PipelineOperator[];
  status: "Created" | "Running" | "Paused" | "Failed";
  created_at: string;
}
```

### Tenant
```typescript
interface Tenant {
  id: string;
  name: string;
  tier: "Free" | "Standard" | "Professional" | "Enterprise";
  quotas: TenantQuotas;
  created_at: string;
}
```

---

## 5. Test Plan - Coverage Mapping

### Current Test Coverage (53 tests in 5 files)

| Test File | Tests | Coverage Area |
|-----------|-------|---------------|
| `demo-ui.spec.ts` | 12 | Hero section, feature cards, navigation between sections |
| `event-ingestion.spec.ts` | 8 | Event generation, UI elements, stats display |
| `metrics-demo.spec.ts` | 14 | Metric cards, refresh, loading states, timestamps |
| `query-demo.spec.ts` | 9 | Query execution, parameters, results display |
| `components.spec.ts` | 10 | UI component test page (buttons, badges, cards) |

### Test Coverage by Section

| Section | Status | Tests | Notes |
|---------|--------|-------|-------|
| Hero/Navigation | ✅ Covered | 12 | `demo-ui.spec.ts` |
| Event Ingestion | ✅ Covered | 8 | `event-ingestion.spec.ts` |
| Query Engine | ✅ Covered | 9 | `query-demo.spec.ts` |
| System Metrics | ✅ Covered | 14 | `metrics-demo.spec.ts` |
| Projections | ⚠️ Partial | 1 | Navigation only (in demo-ui.spec.ts) |
| Security | ⚠️ Partial | 1 | Navigation only (in demo-ui.spec.ts) |
| Time Travel | ⚠️ Partial | 1 | Navigation only (in demo-ui.spec.ts) |
| Analytics | ❌ Not covered | 0 | No dedicated tests |
| Pipelines | ❌ Not covered | 0 | No dedicated tests |

### Recommended Additional Tests

#### Projections (`projections-demo.spec.ts`)
- [ ] Display projection demo UI elements
- [ ] Show all 5 projection types
- [ ] Create a projection
- [ ] Pause/resume projection controls
- [ ] Display projection statistics
- [ ] Handle mock data fallback

#### Security (`security-demo.spec.ts`)
- [ ] Display security demo UI elements
- [ ] Show all 8 security feature cards
- [ ] Navigate between Overview/Tenants/Audit Logs tabs
- [ ] Display tenant list
- [ ] Create new tenant
- [ ] Display audit log entries
- [ ] Handle mock data fallback

#### Time Travel (`time-travel-demo.spec.ts`)
- [ ] Display time travel demo UI elements
- [ ] Show timeline visualization
- [ ] Test playback controls (play, pause, prev, next)
- [ ] Display entity state at selected time
- [ ] Handle mock data fallback

#### Analytics (`analytics-demo.spec.ts`)
- [ ] Display analytics demo UI elements
- [ ] Show all 4 analytics options
- [ ] Execute time series analysis
- [ ] Execute funnel analysis
- [ ] Execute cohort analysis
- [ ] Test statistical aggregations
- [ ] Handle mock data fallback

#### Pipelines (`pipelines-demo.spec.ts`)
- [ ] Display pipelines demo UI elements
- [ ] Show all 10 operator buttons
- [ ] Add operators to pipeline
- [ ] Remove operators from pipeline
- [ ] Create and run pipeline
- [ ] Display pipeline success message
- [ ] Handle mock data fallback

---

## 6. Test Architecture Notes

### Patterns Used

1. **Page Object Model**: `DemoPage` class in `page-objects/DemoPage.ts`
2. **Fixtures**: Custom fixtures in `fixtures/pages.ts` for dependency injection
3. **Fallback Handling**: Tests check for either loaded data OR loading state (graceful degradation)

### Key Gotchas

1. **Metrics API requires authentication** - demo shows "Loading metrics..." when API fails
2. **Use `exact: true`** in `getByRole` to avoid strict mode violations
3. **Use `.first()`** when selecting from potentially multiple matches
4. **Feature cards** render full description in accessible name
5. **Web app title** comes from layout (`acme.ai`), not individual page content

### Running Tests

```bash
# Via turbo (recommended)
turbo run test --filter=@allsource/e2e

# List tests only
turbo run test --filter=@allsource/e2e -- --list

# Note: `bun test` runs bun's test runner which picks up packages/ui tests
# Use `bun run test` (npm script) or turbo for correct Playwright execution
```

---

## 7. Summary

| Metric | Value |
|--------|-------|
| Total Sections | 8 |
| API-Dependent Sections | All 8 (with mock fallbacks) |
| Current Tests | 53 |
| Full Coverage Sections | 4 (Hero, Events, Queries, Metrics) |
| Partial Coverage Sections | 3 (Projections, Security, Time Travel) |
| No Coverage Sections | 2 (Analytics, Pipelines) |
| Recommended Additional Tests | ~30-35 |
