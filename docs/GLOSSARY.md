---
title: "Glossary of Terms"
status: CURRENT
last_updated: 2026-02-02
category: reference
---

# Glossary of Terms

This glossary provides definitions for key terms used throughout the Chronos project. Terms are organized alphabetically within their respective categories.

---

## Architecture Terms

**Application Layer**
The layer in Clean Architecture that contains use cases and orchestrates the flow of data between the domain and infrastructure layers. It implements business logic that coordinates domain entities without containing domain rules itself.

**Clean Architecture**
An architectural pattern that separates software into concentric layers with dependencies pointing inward. The core domain layer has no dependencies on outer layers (application, infrastructure), making the system testable, maintainable, and independent of frameworks. See [Architecture documentation](./ARCHITECTURE.md) for Chronos-specific implementation.

**DDD (Domain-Driven Design)**
A software design approach that focuses on modeling software to match the business domain. It emphasizes ubiquitous language, bounded contexts, and strategic design patterns. Example: The Chronos event store uses DDD concepts like aggregates and entities to model event streams.

**Domain Layer**
The innermost layer in Clean Architecture containing business entities, value objects, and domain logic. It has no dependencies on external frameworks or infrastructure concerns. In Chronos, this includes core event sourcing primitives.

**Infrastructure Layer**
The outermost layer in Clean Architecture that handles external concerns like databases, file systems, and external services. In Chronos, this includes WAL storage, Parquet file handling, and MCP protocol implementation.

**SOLID Principles**
Five design principles for object-oriented programming:
- **S**ingle Responsibility: A class should have one reason to change
- **O**pen/Closed: Open for extension, closed for modification
- **L**iskov Substitution: Subtypes must be substitutable for base types
- **I**nterface Segregation: Many specific interfaces over one general-purpose interface
- **D**ependency Inversion: Depend on abstractions, not concretions

---

## Chronos-Specific Terms

**Control Plane**
The management layer in Chronos responsible for cluster coordination, configuration management, and administrative operations. It handles schema registration, node discovery, and system-wide settings.

**DashMap**
A concurrent hash map implementation used in the Rust components of Chronos for high-performance, thread-safe key-value storage. It provides lock-free reads and fine-grained locking for writes, enabling efficient concurrent access to in-memory event data.

**MCP (Model Context Protocol)**
A protocol for communication between AI models and external tools/services. Chronos implements an MCP server to expose event store operations to AI assistants, enabling natural language interaction with event data. See [MCP Server documentation](./mcp-server/).

**Parquet**
A columnar storage file format optimized for analytics workloads. Chronos uses Parquet for long-term event storage and efficient querying of historical data. Example: Events are periodically compacted from WAL into Parquet files for archival.

**Query Service**
The Chronos component responsible for handling read operations and projections. It provides APIs for querying events, retrieving entity state, and executing complex queries across event streams. See [Query Service documentation](./query-service/).

**Schema Registry**
A centralized service that manages event schemas and ensures schema compatibility across the system. It validates events against registered schemas and handles schema evolution. Example: When a new event type is introduced, its schema must be registered before events can be published.

**TOON (Token-Optimized Object Notation)**
A compact serialization format designed to minimize token usage when communicating with AI models. TOON reduces the size of event payloads while maintaining readability, making it efficient for MCP-based interactions.

**WAL (Write-Ahead Log)**
A persistence mechanism where changes are written to a log before being applied to the main data store. In Chronos, the WAL ensures durability by recording events before acknowledgment, enabling recovery after failures. Example: Each event is appended to the WAL before being indexed in memory.

---

## Event Sourcing Terms

**Aggregate**
A cluster of domain objects treated as a single unit for data changes. An aggregate has a root entity that controls access to its members and ensures invariants are maintained. Example: An `Order` aggregate might contain `OrderItem` entities and enforce rules like "order total cannot be negative."

**Command**
An intent to perform an action that may result in one or more events. Commands are validated and processed by aggregates, which then emit events if the command is accepted. Example: `PlaceOrder` command might result in an `OrderPlaced` event.

**Entity ID**
A unique identifier that distinguishes one entity from another across the system. In Chronos, entity IDs are used to group related events into streams and retrieve entity state. Example: `order-123` identifies a specific order's event stream.

**Event**
An immutable record of something that happened in the system. Events are the source of truth in event sourcing and represent facts that cannot be changed or deleted. Example: `OrderPlaced { order_id: "123", customer_id: "456", total: 99.99 }`

**Event Sourcing**
An architectural pattern where application state is derived from a sequence of events rather than stored directly. Instead of updating records in place, all changes are captured as immutable events, enabling complete audit trails and temporal queries. See [Event Sourcing Guide](./guides/event-sourcing.md).

**Event Store**
A database optimized for storing and retrieving events. It provides append-only storage, stream-based access, and efficient replay capabilities. Chronos implements a distributed event store with WAL, in-memory indexing, and Parquet archival.

**Event Stream**
An ordered sequence of events belonging to a specific entity or aggregate. Streams provide a complete history of an entity's state changes. Example: The stream for `order-123` contains all events that affected that order.

**Projection**
A read model derived from events that represents a specific view of the data. Projections are built by replaying events and can be optimized for specific query patterns. Example: An `OrderSummary` projection might aggregate order data for dashboard display.

**Replay**
The process of re-processing events to rebuild state or projections. Replay enables recovery, debugging, and creation of new views from historical data. Example: After deploying a new projection, events are replayed to populate it with historical data.

**Snapshot**
A point-in-time capture of an aggregate's state used to optimize replay performance. Instead of replaying all events, the system loads the latest snapshot and only replays subsequent events. Example: A snapshot every 100 events reduces replay time significantly.

---

## Technical Terms

**GenServer (Elixir)**
A generic server behavior in Elixir/OTP that abstracts the common client-server pattern. GenServers handle synchronous and asynchronous messages, maintain state, and integrate with OTP supervision trees. Example: The Chronos MCP server uses GenServer to manage connection state.

**JWT (JSON Web Token)**
A compact, URL-safe token format for securely transmitting claims between parties. Chronos uses JWTs for authentication and authorization, encoding user identity and permissions. Example: API requests include a JWT in the Authorization header.

**OTLP (OpenTelemetry Protocol)**
A standard protocol for transmitting telemetry data (traces, metrics, logs) between services and observability backends. Chronos uses OTLP to export distributed traces and metrics to monitoring systems.

**OTP (Open Telecom Platform)**
A set of Erlang libraries and design principles for building concurrent, fault-tolerant systems. OTP provides supervision trees, behaviors (GenServer, Supervisor), and patterns for building reliable distributed applications. The Elixir components of Chronos are built on OTP.

**RBAC (Role-Based Access Control)**
A security model where permissions are assigned to roles, and users are assigned to roles. Chronos uses RBAC to control access to event streams and administrative operations. Example: A `reader` role might have permission to query events but not write them.

---

## See Also

- [Architecture Overview](./ARCHITECTURE.md)
- [Getting Started Guide](./guides/getting-started.md)
- [API Reference](./api/)
- [Contributing Guide](./CONTRIBUTING.md)
