(ns allsource.application.usecases.projection-executor
  "Projection executor implementation.
   Manages lifecycle and execution of projections."
  (:require [allsource.domain.entities.projection :as p]
            [allsource.domain.protocols.projection-executor :as pe])
  (:import [java.time Instant]))

;; ============================================================================
;; In-Memory State Store (Simple Implementation)
;; ============================================================================

(defrecord InMemoryStateStore [state-atom]
  pe/StateStore

  (save-state [this projection-name entity-id state version]
    (try
      (swap! state-atom assoc-in [projection-name entity-id]
             {:state state :version version :updated-at (Instant/now)})
      {:success true}
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (load-state [this projection-name entity-id]
    (get-in @state-atom [projection-name entity-id :state]))

  (delete-state [this projection-name entity-id]
    (try
      (swap! state-atom update projection-name dissoc entity-id)
      {:success true}
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (save-snapshot [this snapshot]
    (let [snapshot-id (str (java.util.UUID/randomUUID))]
      (swap! state-atom assoc-in [:snapshots (:projection-name snapshot) (:entity-id snapshot)]
             (assoc snapshot :snapshot-id snapshot-id))
      {:success true :snapshot-id snapshot-id}))

  (load-snapshot [this projection-name entity-id]
    (get-in @state-atom [:snapshots projection-name entity-id]))

  (list-all-states [this projection-name]
    (let [states (get @state-atom projection-name)]
      (for [[entity-id data] states
            :when (not= entity-id :snapshots)]
        [entity-id (:state data)]))))

(defn create-state-store
  "Create a new in-memory state store."
  []
  (->InMemoryStateStore (atom {})))

;; ============================================================================
;; Projection Registry (In-Memory)
;; ============================================================================

(defrecord InMemoryRegistry [registry-atom]
  pe/ProjectionRegistry

  (register-projection [this projection-def]
    (if-not (p/valid-projection? projection-def)
      {:success false :error "Invalid projection definition"}
      (let [name (p/get-name projection-def)]
        (swap! registry-atom assoc name projection-def)
        {:success true :projection-name name})))

  (unregister-projection [this projection-name]
    (swap! registry-atom dissoc projection-name)
    {:success true})

  (get-projection-def [this projection-name]
    (get @registry-atom projection-name))

  (list-projections [this]
    (vals @registry-atom)))

(defn create-registry
  "Create a new in-memory projection registry."
  []
  (->InMemoryRegistry (atom {})))

;; ============================================================================
;; Projection Executor Implementation
;; ============================================================================

(defrecord ProjectionExecutorImpl [registry state-store running-projections-atom]
  pe/ProjectionExecutor

  (start-projection [this projection-def]
    (if-not (p/valid-projection? projection-def)
      {:status :error :error "Invalid projection definition"}
      (let [name (p/get-name projection-def)]
        (if (contains? @running-projections-atom name)
          {:status :error :error (str "Projection " name " is already running")}
          (do
            ;; Register the projection
            (pe/register-projection registry projection-def)
            ;; Mark as running
            (swap! running-projections-atom assoc name
                   {:definition projection-def
                    :status :running
                    :started-at (Instant/now)
                    :last-processed-event nil
                    :entities-count 0})
            {:status :started :projection-name name})))))

  (stop-projection [this projection-name]
    (if-not (contains? @running-projections-atom projection-name)
      {:status :error :error (str "Projection " projection-name " is not running")}
      (do
        (swap! running-projections-atom update projection-name assoc
               :status :stopped
               :stopped-at (Instant/now))
        (swap! running-projections-atom dissoc projection-name)
        {:status :stopped :projection-name projection-name})))

  (reload-projection [this projection-def]
    (let [name (p/get-name projection-def)]
      (if (contains? @running-projections-atom name)
        ;; Stop and restart with new definition
        (do
          (pe/stop-projection this name)
          (pe/start-projection this projection-def)
          {:status :reloaded :projection-name name})
        ;; Not running, just start it
        (do
          (pe/start-projection this projection-def)
          {:status :started :projection-name name}))))

  (get-projection-state [this projection-name entity-id]
    (when (contains? @running-projections-atom projection-name)
      (pe/load-state state-store projection-name entity-id)))

  (get-projection-status [this projection-name]
    (when-let [proj-info (get @running-projections-atom projection-name)]
      {:status (:status proj-info)
       :version (p/get-version (:definition proj-info))
       :last-processed-event (:last-processed-event proj-info)
       :entities-count (:entities-count proj-info)
       :started-at (:started-at proj-info)})))

;; ============================================================================
;; Event Processing Functions
;; ============================================================================

(defn process-event
  "Process a single event through a projection.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection
   - event: Event map to process

   Returns: {:status :success} or {:status :error :error <message>}"
  [executor projection-name event]
  (let [running-projections (:running-projections-atom executor)
        state-store (:state-store executor)]
    (if-let [proj-info (get @running-projections projection-name)]
      (try
        (let [projection-def (:definition proj-info)
              entity-id (or (:entity-id event) "global")
              ;; Load current state or use initial state
              current-state (or (pe/load-state state-store projection-name entity-id)
                                (p/initialize-state projection-def))
              ;; Apply event to state
              new-state (p/apply-event projection-def current-state event)]
          ;; Save new state
          (pe/save-state state-store projection-name entity-id new-state
                         (p/get-version projection-def))
          ;; Update projection info
          (swap! running-projections update projection-name
                 (fn [info]
                   (-> info
                       (assoc :last-processed-event (:timestamp event))
                       (update :entities-count (fn [c] (or c 0))))))
          {:status :success})
        (catch Exception e
          {:status :error :error (.getMessage e)}))
      {:status :error :error (str "Projection " projection-name " is not running")})))

(defn create-snapshot
  "Create a snapshot of projection state for an entity.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection
   - entity-id: Entity ID

   Returns: {:success true :snapshot-id <id>} or {:success false :error <message>}"
  [executor projection-name entity-id]
  (let [state-store (:state-store executor)
        running-projections (:running-projections-atom executor)]
    (if-let [proj-info (get @running-projections projection-name)]
      (if-let [current-state (pe/load-state state-store projection-name entity-id)]
        (let [projection-def (:definition proj-info)
              snapshot (p/snapshot-state projection-name entity-id current-state
                                         (p/get-version projection-def))]
          (pe/save-snapshot state-store snapshot))
        {:success false :error (str "No state found for entity " entity-id)})
      {:success false :error (str "Projection " projection-name " is not running")})))

(defn restore-from-snapshot
  "Restore projection state from the latest snapshot.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection
   - entity-id: Entity ID

   Returns: {:success true} or {:success false :error <message>}"
  [executor projection-name entity-id]
  (let [state-store (:state-store executor)
        running-projections (:running-projections-atom executor)]
    (if-let [proj-info (get @running-projections projection-name)]
      (if-let [snapshot (pe/load-snapshot state-store projection-name entity-id)]
        (let [restored-state (p/restore-from-snapshot snapshot)
              version (:version snapshot)]
          (pe/save-state state-store projection-name entity-id restored-state version)
          {:success true})
        {:success false :error (str "No snapshot found for entity " entity-id)})
      {:success false :error (str "Projection " projection-name " is not running")})))

;; ============================================================================
;; Constructor
;; ============================================================================

(defn create-executor
  "Create a new projection executor instance.

   Options:
   - :state-store - Custom state store (default: in-memory)
   - :registry - Custom registry (default: in-memory)

   Returns: ProjectionExecutorImpl instance"
  ([]
   (create-executor {}))
  ([{:keys [state-store registry]}]
   (->ProjectionExecutorImpl
     (or registry (create-registry))
     (or state-store (create-state-store))
     (atom {}))))

;; ============================================================================
;; Batch Processing
;; ============================================================================

(defn process-events
  "Process multiple events through a projection.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection
   - events: Sequence of events

   Returns: {:success true :processed-count <n>} or {:success false :error <message>}"
  [executor projection-name events]
  (try
    (let [results (doall (map #(process-event executor projection-name %) events))
          errors (filter #(= :error (:status %)) results)]
      (if (empty? errors)
        {:success true :processed-count (count events)}
        {:success false
         :processed-count (- (count events) (count errors))
         :errors errors}))
    (catch Exception e
      {:success false :error (.getMessage e)})))

;; ============================================================================
;; Query Functions
;; ============================================================================

(defn get-all-entity-states
  "Get all entity states for a projection.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection

   Returns: Map of entity-id => state"
  [executor projection-name]
  (let [state-store (:state-store executor)]
    (into {} (pe/list-all-states state-store projection-name))))

(defn get-projection-metrics
  "Get metrics for a projection.

   Parameters:
   - executor: ProjectionExecutorImpl instance
   - projection-name: Name of the projection

   Returns: Map with metrics"
  [executor projection-name]
  (let [running-projections (:running-projections-atom executor)
        state-store (:state-store executor)]
    (when-let [proj-info (get @running-projections projection-name)]
      (let [all-states (pe/list-all-states state-store projection-name)]
        {:projection-name projection-name
         :version (p/get-version (:definition proj-info))
         :status (:status proj-info)
         :started-at (:started-at proj-info)
         :last-processed-event (:last-processed-event proj-info)
         :entities-count (count all-states)}))))
