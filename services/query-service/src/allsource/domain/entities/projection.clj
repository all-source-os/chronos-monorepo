(ns allsource.domain.entities.projection
  "Domain entities for projections.
   A projection transforms event streams into queryable read models."
  (:import [java.time Instant]))

;; ============================================================================
;; Projection Definition Entity
;; ============================================================================

(defrecord ProjectionDefinition [name version initial-state project-fn metadata])

(defrecord ProjectionSnapshot [projection-name entity-id state version timestamp])

;; ============================================================================
;; Validation Functions (Pure Domain Logic)
;; ============================================================================

(defn valid-version?
  "Check if version is valid (positive integer)."
  [version]
  (and (integer? version)
       (pos? version)))

(defn valid-name?
  "Check if projection name is valid (must be keyword)."
  [name]
  (keyword? name))

(defn valid-projection?
  "Validate a complete projection definition."
  [projection]
  (and (instance? ProjectionDefinition projection)
       (valid-name? (:name projection))
       (valid-version? (:version projection))
       (some? (:initial-state projection))
       (fn? (:project-fn projection))))

;; ============================================================================
;; Constructor Functions
;; ============================================================================

(defn make-projection
  "Create a new projection definition.

   Parameters:
   - name: Keyword identifier for the projection
   - version: Integer version number (for migration tracking)
   - initial-state: Initial state value (any data structure)
   - project-fn: Function (state, event) => new-state
   - metadata: Optional metadata map

   Example:
   (make-projection
     :name :user-statistics
     :version 1
     :initial-state {:count 0 :total-spent 0.0}
     :project-fn (fn [state event]
                   (case (:event-type event)
                     \"order.placed\" (-> state
                                          (update :count inc)
                                          (update :total-spent + (get-in event [:payload :amount])))
                     state)))"
  [& {:keys [name version initial-state project-fn metadata]
      :or {metadata {}}}]
  (map->ProjectionDefinition
    {:name name
     :version version
     :initial-state initial-state
     :project-fn project-fn
     :metadata metadata}))

(defn make-snapshot
  "Create a projection state snapshot.

   Parameters:
   - projection-name: Name of the projection
   - entity-id: ID of the entity this state belongs to
   - state: Current state value
   - version: Projection version that created this state
   - timestamp: When this snapshot was taken"
  [& {:keys [projection-name entity-id state version timestamp]}]
  (map->ProjectionSnapshot
    {:projection-name projection-name
     :entity-id entity-id
     :state state
     :version version
     :timestamp timestamp}))

;; ============================================================================
;; Projection Application (Pure Functions)
;; ============================================================================

(defn apply-event
  "Apply a single event to projection state.

   Parameters:
   - projection: ProjectionDefinition
   - state: Current state
   - event: Event to apply

   Returns: New state after applying event"
  [projection state event]
  (let [project-fn (:project-fn projection)]
    (project-fn state event)))

(defn apply-events
  "Apply multiple events to projection state.

   Parameters:
   - projection: ProjectionDefinition
   - state: Initial state
   - events: Sequence of events to apply

   Returns: Final state after applying all events"
  [projection state events]
  (reduce (partial apply-event projection) state events))

(defn initialize-state
  "Get initial state for a projection."
  [projection]
  (:initial-state projection))

;; ============================================================================
;; State Migration
;; ============================================================================

(defn migrate-state
  "Migrate projection state using a migration function.

   Parameters:
   - old-state: State from previous version
   - migration-fn: Function to transform old state to new state

   Returns: Migrated state"
  [old-state migration-fn]
  (migration-fn old-state))

;; ============================================================================
;; Accessors
;; ============================================================================

(defn get-name
  "Get projection name."
  [projection]
  (:name projection))

(defn get-version
  "Get projection version."
  [projection]
  (:version projection))

(defn get-initial-state
  "Get projection initial state."
  [projection]
  (:initial-state projection))

(defn get-metadata
  "Get projection metadata."
  [projection]
  (:metadata projection))

;; ============================================================================
;; State Snapshot Management
;; ============================================================================

(defn snapshot-state
  "Create a snapshot of current projection state.

   Parameters:
   - projection-name: Name of the projection
   - entity-id: Entity ID
   - state: Current state
   - version: Projection version

   Returns: ProjectionSnapshot"
  [projection-name entity-id state version]
  (make-snapshot
    :projection-name projection-name
    :entity-id entity-id
    :state state
    :version version
    :timestamp (Instant/now)))

(defn restore-from-snapshot
  "Restore state from a snapshot."
  [snapshot]
  (:state snapshot))

;; ============================================================================
;; Example Projections (for documentation)
;; ============================================================================

(comment
  ;; Example: User Statistics Projection
  (def user-stats-projection
    (make-projection
      :name :user-statistics
      :version 1
      :initial-state {:user-count 0
                      :total-orders 0
                      :total-revenue 0.0}
      :project-fn (fn [state event]
                    (case (:event-type event)
                      "user.created"
                      (update state :user-count inc)

                      "order.placed"
                      (-> state
                          (update :total-orders inc)
                          (update :total-revenue + (get-in event [:payload :amount])))

                      state))))

  ;; Apply events to projection
  (def events [{:event-type "user.created" :entity-id "user-1"}
               {:event-type "order.placed" :entity-id "user-1" :payload {:amount 100.0}}
               {:event-type "order.placed" :entity-id "user-1" :payload {:amount 50.0}}])

  (apply-events user-stats-projection
                (initialize-state user-stats-projection)
                events)
  ;; => {:user-count 1 :total-orders 2 :total-revenue 150.0}

  ;; Create snapshot
  (snapshot-state :user-statistics
                  "user-1"
                  {:user-count 1 :total-orders 2 :total-revenue 150.0}
                  1)
  )
