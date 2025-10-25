(ns allsource.domain.protocols.projection-executor
  "Protocols for projection execution and state management.
   Defines abstractions for running projections against event streams.")

;; ============================================================================
;; Projection Execution Protocol
;; ============================================================================

(defprotocol ProjectionExecutor
  "Protocol for executing and managing projections.

   A projection executor is responsible for:
   - Starting/stopping projections
   - Applying events to projection state
   - Managing projection lifecycle
   - Handling state snapshots"

  (start-projection [this projection-def]
    "Start a projection with the given definition.

     Parameters:
     - projection-def: ProjectionDefinition entity

     Returns: {:status :started :projection-name <name>}")

  (stop-projection [this projection-name]
    "Stop a running projection.

     Parameters:
     - projection-name: Keyword identifier of projection

     Returns: {:status :stopped :projection-name <name>}")

  (reload-projection [this projection-def]
    "Reload a projection with new definition (hot reload).

     Parameters:
     - projection-def: New ProjectionDefinition entity

     Returns: {:status :reloaded :projection-name <name>}")

  (get-projection-state [this projection-name entity-id]
    "Get current state for a specific entity in the projection.

     Parameters:
     - projection-name: Keyword identifier of projection
     - entity-id: ID of the entity

     Returns: Current state map or nil if not found")

  (get-projection-status [this projection-name]
    "Get status information about a projection.

     Parameters:
     - projection-name: Keyword identifier of projection

     Returns: {:status :running/:stopped/:error
               :version <version>
               :last-processed-event <timestamp>
               :entities-count <count>}"))

;; ============================================================================
;; State Store Protocol
;; ============================================================================

(defprotocol StateStore
  "Protocol for persisting and retrieving projection state.

   Implementations might use:
   - In-memory maps (for development/testing)
   - PostgreSQL (for production)
   - Redis (for caching)
   - DynamoDB (for serverless)"

  (save-state [this projection-name entity-id state version]
    "Save projection state for an entity.

     Parameters:
     - projection-name: Keyword identifier
     - entity-id: Entity ID
     - state: State map
     - version: Projection version

     Returns: {:success true} or {:success false :error <error>}")

  (load-state [this projection-name entity-id]
    "Load projection state for an entity.

     Parameters:
     - projection-name: Keyword identifier
     - entity-id: Entity ID

     Returns: State map or nil if not found")

  (delete-state [this projection-name entity-id]
    "Delete projection state for an entity.

     Parameters:
     - projection-name: Keyword identifier
     - entity-id: Entity ID

     Returns: {:success true} or {:success false :error <error>}")

  (save-snapshot [this snapshot]
    "Save a projection state snapshot.

     Parameters:
     - snapshot: ProjectionSnapshot entity

     Returns: {:success true :snapshot-id <id>}")

  (load-snapshot [this projection-name entity-id]
    "Load latest snapshot for entity.

     Parameters:
     - projection-name: Keyword identifier
     - entity-id: Entity ID

     Returns: ProjectionSnapshot entity or nil")

  (list-all-states [this projection-name]
    "List all entity states for a projection.

     Parameters:
     - projection-name: Keyword identifier

     Returns: Sequence of [entity-id state] tuples"))

;; ============================================================================
;; Projection Registry Protocol
;; ============================================================================

(defprotocol ProjectionRegistry
  "Protocol for managing registered projections.

   Keeps track of all available projections and their definitions."

  (register-projection [this projection-def]
    "Register a projection definition.

     Parameters:
     - projection-def: ProjectionDefinition entity

     Returns: {:success true :projection-name <name>}")

  (unregister-projection [this projection-name]
    "Unregister a projection.

     Parameters:
     - projection-name: Keyword identifier

     Returns: {:success true}")

  (get-projection-def [this projection-name]
    "Get registered projection definition.

     Parameters:
     - projection-name: Keyword identifier

     Returns: ProjectionDefinition entity or nil")

  (list-projections [this]
    "List all registered projections.

     Returns: Sequence of ProjectionDefinition entities"))

;; ============================================================================
;; Event Stream Protocol
;; ============================================================================

(defprotocol EventStream
  "Protocol for consuming events from the event store.

   Provides a stream of events for projections to process."

  (subscribe-to-events [this projection-name callback]
    "Subscribe to event stream for a projection.

     Parameters:
     - projection-name: Keyword identifier
     - callback: Function (event) => void

     Returns: Subscription handle")

  (unsubscribe [this subscription]
    "Unsubscribe from event stream.

     Parameters:
     - subscription: Subscription handle from subscribe-to-events")

  (replay-events [this projection-name from-timestamp callback]
    "Replay historical events for rebuilding projection state.

     Parameters:
     - projection-name: Keyword identifier
     - from-timestamp: Start timestamp (or nil for all events)
     - callback: Function (event) => void

     Returns: {:success true :events-processed <count>}"))
