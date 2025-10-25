# AllSource Query Service (Clojure)

**Phase 2 v1.3** - Query DSL + Interactive REPL

A Clojure-based query layer for the AllSource event store, featuring:
- 🎯 Declarative Query DSL
- 🏗️ Clean Architecture (Domain → Application → Infrastructure)
- 🔌 Component-based dependency injection
- 🚀 Interactive REPL with helper functions
- 📊 Zero-copy query execution via HTTP to Rust core

---

## 🏗️ Clean Architecture

```
src/allsource/
├── domain/              ✅ Layer 1: Pure business logic
│   ├── entities/       - Query, Predicate, Aggregation
│   └── protocols/      - QueryExecutor, QueryOptimizer
├── application/         ✅ Layer 2: Use cases & DSL
│   ├── dsl.clj         - User-facing query API
│   └── usecases/       - Future: query optimization
└── infrastructure/      ✅ Layer 3: External adapters
    ├── adapters/       - HTTP client to Rust core
    ├── web/            - Future: REST API
    └── config/         - Component system
```

**Dependency Direction**: Infrastructure → Application → Domain (inward only)

---

## 🚀 Quick Start

### 1. Start the REPL

```bash
cd services/query-service
clj -M:dev:repl
```

### 2. Start the System

```clojure
user=> (start!)
✓ System started
```

### 3. Run Your First Query

```clojure
;; Get 10 most recent events
user=> (pp (recent 10))

;; Get events by type
user=> (pp (by-type "user.created" 20))

;; Build custom query
user=> (pp (execute!
         (-> (dsl/from-events)
             (dsl/where [:= :event-type "order.placed"])
             (dsl/since (dsl/days-ago 7))
             (dsl/limit 100))))
```

---

## 📖 Query DSL Guide

### Simple Query (Map Syntax)

```clojure
(require '[allsource.application.dsl :as dsl])

(dsl/query
  {:select [:entity-id :event-type :timestamp]
   :from :events
   :where [:= :event-type "user.created"]
   :limit 100})
```

### Fluent Query Building

```clojure
(-> (dsl/from-events)
    (dsl/select [:entity-id :event-type])
    (dsl/where [:= :event-type "user.created"])
    (dsl/order-by-timestamp :desc)
    (dsl/limit 100))
```

### Complex Queries with AND/OR

```clojure
(dsl/query
  {:from :events
   :where [:and
           [:= :event-type "order.placed"]
           [:> :timestamp (dsl/days-ago 7)]
           [:or
            [:> [:payload :amount] 1000]
            [:contains [:payload :tags] "premium"]]]
   :order-by [[:timestamp :desc]]
   :limit 50})
```

### Time-Based Queries

```clojure
;; Events from last 7 days
(-> (dsl/from-events)
    (dsl/since (dsl/days-ago 7))
    (dsl/limit 1000))

;; Events in time range
(-> (dsl/from-events)
    (dsl/time-range (dsl/days-ago 30) (dsl/now))
    (dsl/limit 5000))

;; Events from specific hours
(-> (dsl/from-events)
    (dsl/since (dsl/hours-ago 2))
    (dsl/order-by-timestamp))
```

---

## 🎛️ REPL Helper Functions

### System Management

```clojure
(start!)              ; Start the system
(stop!)               ; Stop the system
(restart!)            ; Restart the system
(status)              ; Check system status
```

### Quick Queries

```clojure
(recent 10)                      ; Get 10 most recent events
(by-type "user.created" 50)      ; Get 50 events by type
(by-entity "user-123")           ; Get all events for entity
(since (dsl/days-ago 7) 100)     ; Events from last 7 days
(today 100)                      ; Today's events
```

### Analysis

```clojure
(count-by-type 30)               ; Count events by type (last 30 days)
```

### Pretty Printing

```clojure
(pp data)                        ; Pretty print with fipp
(show events 10)                 ; Show first 10 events
```

---

## 🧪 Comparison Operators

```clojure
(dsl/eq :field value)            ; Equal
(dsl/ne :field value)            ; Not equal
(dsl/gt :field value)            ; Greater than
(dsl/gte :field value)           ; Greater than or equal
(dsl/lt :field value)            ; Less than
(dsl/lte :field value)           ; Less than or equal
(dsl/contains? :field substring) ; String contains
(dsl/in? :field [v1 v2 v3])      ; IN operator
(dsl/between :field lower upper) ; BETWEEN operator
```

## 🔗 Logical Operators

```clojure
(dsl/and pred1 pred2 pred3)      ; Combine with AND
(dsl/or pred1 pred2)             ; Combine with OR
(dsl/not pred)                   ; Negate predicate
```

---

## 📊 Query Execution

### Synchronous Execution

```clojure
(require '[allsource.domain.protocols.query-executor :as qe])

(let [client (sys/get-query-client)
      query (dsl/query {:from :events :limit 100})]
  (qe/execute-query client query))
```

### Asynchronous Execution

```clojure
(qe/execute-query-async
  client
  query
  (fn [result]
    (if (:success result)
      (println "Got" (count (:results result)) "events")
      (println "Error:" (:error result)))))
```

### Streaming Results

```clojure
(let [results (qe/stream-query-results client query)]
  (doseq [event (take 100 results)]
    (println (:event-type event))))
```

---

## 🧱 Architecture Principles (SOLID)

### Single Responsibility Principle (SRP)
- `Query` entity: Only represents a query
- `RustCoreClient`: Only communicates with Rust core
- `query-compiler`: Only translates queries

### Open/Closed Principle (OCP)
- New query executors via `QueryExecutor` protocol
- New query optimizers via `QueryOptimizer` protocol

### Liskov Substitution Principle (LSP)
- Any `QueryExecutor` implementation is interchangeable
- Mock, HTTP, or local executors all conform to same protocol

### Interface Segregation Principle (ISP)
- Small focused protocols: `QueryExecutor`, `QueryOptimizer`, `QueryValidator`
- Not one large "QueryService" protocol

### Dependency Inversion Principle (DIP)
- Application depends on `QueryExecutor` protocol
- Infrastructure provides `RustCoreClient` implementation
- Easy to swap or mock for testing

---

## 🧪 Testing

### Run Tests

```bash
clj -M:test
```

### Test Structure

```
test/allsource/
├── domain/
│   └── entities/
│       └── query_test.clj
├── application/
│   └── dsl_test.clj
└── infrastructure/
    └── adapters/
        ├── query_compiler_test.clj
        └── rust_core_client_test.clj
```

---

## 📦 Dependencies

- **clojure** (1.11.1) - Core language
- **component** - Dependency injection
- **clj-http** - HTTP client
- **cheshire** - JSON encoding/decoding
- **malli** - Data validation
- **fipp** - Pretty printing
- **java-time** - Date/time handling

---

## 🛠️ Development

### Start REPL with nREPL

```bash
clj -M:repl
```

Then connect from your editor (VS Code, Emacs, IntelliJ)

### Reload Code

```clojure
(require '[allsource.application.dsl :as dsl] :reload-all)
```

### Hot Reload System

```clojure
(restart!)
```

---

## 🌍 Configuration

Set environment variables:

```bash
export ALLSOURCE_AUTH_TOKEN="your-jwt-token"
export RUST_CORE_URL="http://localhost:8080"
```

Or configure in code:

```clojure
(sys/start-system! :production)
```

---

## 📚 Examples

### Example 1: Recent Orders

```clojure
(-> (dsl/from-events)
    (dsl/where [:= :event-type "order.placed"])
    (dsl/since (dsl/days-ago 7))
    (dsl/order-by-timestamp :desc)
    (dsl/limit 100)
    execute!
    pp)
```

### Example 2: High-Value Orders

```clojure
(dsl/query
  {:from :events
   :where [:and
           [:= :event-type "order.placed"]
           [:> [:payload :amount] 5000]]
   :order-by [[:timestamp :desc]]
   :limit 50})
```

### Example 3: User Activity

```clojure
(defn user-activity [user-id days]
  (-> (dsl/from-events)
      (dsl/where [:= :entity-id user-id])
      (dsl/since (dsl/days-ago days))
      (dsl/order-by-timestamp :asc)))

(pp (execute! (user-activity "user-123" 30)))
```

---

## 🔜 Roadmap

### v1.3 (Current) ✅
- [x] Query DSL with declarative syntax
- [x] HTTP client to Rust core
- [x] Interactive REPL environment
- [x] Clean Architecture implementation
- [x] Component-based DI

### v1.4 (Next - Q2 2026)
- [ ] Projection management
- [ ] Hot-reloadable projections
- [ ] Projection state persistence

### v1.5 (Q2-Q3 2026)
- [ ] Event processing pipelines
- [ ] Composable transformations
- [ ] Side-effect handling

### v1.6 (Q3 2026)
- [ ] Analytics engine
- [ ] Time-series aggregations
- [ ] Statistical functions

---

## 📄 License

MIT License - see LICENSE file

---

## 🤝 Contributing

We follow Clean Architecture and SOLID principles. See [CONTRIBUTING.md](../../CONTRIBUTING.md).

---

**Status**: ✅ Phase 2 v1.3 Complete (Query DSL + REPL)
**Next**: v1.4 Projection Management

