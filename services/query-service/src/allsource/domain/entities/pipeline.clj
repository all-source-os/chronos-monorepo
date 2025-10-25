(ns allsource.domain.entities.pipeline
  "Domain entities for event processing pipelines.
   A pipeline transforms event streams through composable operators.")

;; ============================================================================
;; Pipeline Operator Types
;; ============================================================================

(def operator-types
  "Valid operator types for pipeline stages."
  #{:filter        ; Filter events based on predicate
    :map          ; Transform events
    :flat-map     ; Transform and flatten
    :enrich       ; Add data to events
    :window       ; Window events by time or count
    :batch        ; Batch events
    :throttle     ; Rate limiting
    :deduplicate  ; Remove duplicates
    :partition    ; Partition by key
    :aggregate})  ; Aggregate events

;; ============================================================================
;; Pipeline Operator Entity
;; ============================================================================

(defrecord PipelineOperator
  [type           ; Operator type (keyword from operator-types)
   name           ; Operator name (for debugging/metrics)
   operator-fn    ; Function to apply
   config         ; Operator-specific configuration
   metadata])     ; Additional metadata

(defrecord Pipeline
  [name           ; Pipeline name
   version        ; Pipeline version
   operators      ; Vector of PipelineOperator
   config         ; Pipeline-level configuration
   metadata])     ; Additional metadata

;; ============================================================================
;; Window Configuration
;; ============================================================================

(defrecord WindowConfig
  [type          ; :tumbling, :sliding, :session
   size          ; Window size (milliseconds or count)
   slide         ; Slide interval (for sliding windows)
   timeout])     ; Session timeout (for session windows)

;; ============================================================================
;; Backpressure Configuration
;; ============================================================================

(defrecord BackpressureConfig
  [strategy       ; :drop, :buffer, :block
   buffer-size    ; Buffer size (for :buffer strategy)
   timeout-ms])   ; Timeout (for :block strategy)

;; ============================================================================
;; Validation Functions
;; ============================================================================

(defn valid-operator-type?
  "Check if operator type is valid."
  [type]
  (contains? operator-types type))

(defn valid-operator?
  "Validate a pipeline operator."
  [operator]
  (and (instance? PipelineOperator operator)
       (valid-operator-type? (:type operator))
       (keyword? (:name operator))
       (fn? (:operator-fn operator))))

(defn valid-pipeline?
  "Validate a complete pipeline definition."
  [pipeline]
  (and (instance? Pipeline pipeline)
       (keyword? (:name pipeline))
       (pos? (:version pipeline))
       (vector? (:operators pipeline))
       (every? valid-operator? (:operators pipeline))))

;; ============================================================================
;; Constructor Functions
;; ============================================================================

(defn make-operator
  "Create a pipeline operator.

   Parameters:
   - type: Operator type keyword
   - name: Operator name
   - operator-fn: Function to apply
   - config: Optional configuration map
   - metadata: Optional metadata map

   Example:
   (make-operator
     :type :filter
     :name :user-events-only
     :operator-fn (fn [event] (= \"user.created\" (:event-type event)))
     :config {:description \"Filter for user events\"})"
  [& {:keys [type name operator-fn config metadata]
      :or {config {} metadata {}}}]
  (map->PipelineOperator
    {:type type
     :name name
     :operator-fn operator-fn
     :config config
     :metadata metadata}))

(defn make-pipeline
  "Create a pipeline.

   Parameters:
   - name: Pipeline name
   - version: Pipeline version
   - operators: Vector of PipelineOperator
   - config: Optional configuration map
   - metadata: Optional metadata map"
  [& {:keys [name version operators config metadata]
      :or {operators [] config {} metadata {}}}]
  (map->Pipeline
    {:name name
     :version version
     :operators operators
     :config config
     :metadata metadata}))

(defn make-window-config
  "Create window configuration.

   Parameters:
   - type: Window type (:tumbling, :sliding, :session)
   - size: Window size (ms or count)
   - slide: Slide interval (for sliding windows)
   - timeout: Session timeout (for session windows)"
  [& {:keys [type size slide timeout]}]
  (map->WindowConfig
    {:type type
     :size size
     :slide slide
     :timeout timeout}))

(defn make-backpressure-config
  "Create backpressure configuration.

   Parameters:
   - strategy: Backpressure strategy (:drop, :buffer, :block)
   - buffer-size: Buffer size (for :buffer strategy)
   - timeout-ms: Timeout (for :block strategy)"
  [& {:keys [strategy buffer-size timeout-ms]
      :or {buffer-size 1000 timeout-ms 5000}}]
  (map->BackpressureConfig
    {:strategy strategy
     :buffer-size buffer-size
     :timeout-ms timeout-ms}))

;; ============================================================================
;; Pipeline Composition
;; ============================================================================

(defn add-operator
  "Add an operator to a pipeline.

   Parameters:
   - pipeline: Pipeline entity
   - operator: PipelineOperator to add

   Returns: Updated pipeline"
  [pipeline operator]
  (update pipeline :operators conj operator))

(defn remove-operator
  "Remove an operator from a pipeline by name.

   Parameters:
   - pipeline: Pipeline entity
   - operator-name: Name of operator to remove

   Returns: Updated pipeline"
  [pipeline operator-name]
  (update pipeline :operators
          (fn [ops]
            (filterv #(not= operator-name (:name %)) ops))))

(defn replace-operator
  "Replace an operator in a pipeline.

   Parameters:
   - pipeline: Pipeline entity
   - operator-name: Name of operator to replace
   - new-operator: New operator

   Returns: Updated pipeline"
  [pipeline operator-name new-operator]
  (update pipeline :operators
          (fn [ops]
            (mapv #(if (= operator-name (:name %))
                     new-operator
                     %)
                  ops))))

;; ============================================================================
;; Operator Factories (Common Operators)
;; ============================================================================

(defn filter-operator
  "Create a filter operator.

   Parameters:
   - name: Operator name
   - predicate-fn: Function (event) => boolean"
  [name predicate-fn]
  (make-operator
    :type :filter
    :name name
    :operator-fn predicate-fn
    :config {:description "Filter events by predicate"}))

(defn map-operator
  "Create a map operator.

   Parameters:
   - name: Operator name
   - transform-fn: Function (event) => transformed-event"
  [name transform-fn]
  (make-operator
    :type :map
    :name name
    :operator-fn transform-fn
    :config {:description "Transform events"}))

(defn enrich-operator
  "Create an enrich operator.

   Parameters:
   - name: Operator name
   - enrich-fn: Function (event) => enriched-event"
  [name enrich-fn]
  (make-operator
    :type :enrich
    :name name
    :operator-fn enrich-fn
    :config {:description "Enrich events with additional data"}))

(defn window-operator
  "Create a window operator.

   Parameters:
   - name: Operator name
   - window-config: WindowConfig entity
   - aggregate-fn: Function (events-in-window) => result"
  [name window-config aggregate-fn]
  (make-operator
    :type :window
    :name name
    :operator-fn aggregate-fn
    :config {:window window-config
             :description "Window events"}))

(defn batch-operator
  "Create a batch operator.

   Parameters:
   - name: Operator name
   - batch-size: Number of events per batch
   - batch-fn: Function (event-batch) => result"
  [name batch-size batch-fn]
  (make-operator
    :type :batch
    :name name
    :operator-fn batch-fn
    :config {:batch-size batch-size
             :description "Batch events"}))

;; ============================================================================
;; Accessors
;; ============================================================================

(defn get-name
  "Get pipeline name."
  [pipeline]
  (:name pipeline))

(defn get-version
  "Get pipeline version."
  [pipeline]
  (:version pipeline))

(defn get-operators
  "Get pipeline operators."
  [pipeline]
  (:operators pipeline))

(defn get-operator-by-name
  "Get operator by name."
  [pipeline operator-name]
  (first (filter #(= operator-name (:name %))
                 (:operators pipeline))))

(defn get-metadata
  "Get pipeline metadata."
  [pipeline]
  (:metadata pipeline))

;; ============================================================================
;; Pipeline Metrics
;; ============================================================================

(defrecord PipelineMetrics
  [pipeline-name
   total-processed    ; Total events processed
   total-filtered     ; Total events filtered out
   total-errors       ; Total errors
   throughput         ; Events per second
   latency-p50        ; 50th percentile latency (ms)
   latency-p95        ; 95th percentile latency (ms)
   latency-p99])      ; 99th percentile latency (ms)

(defn make-metrics
  "Create pipeline metrics."
  [& {:keys [pipeline-name total-processed total-filtered total-errors
             throughput latency-p50 latency-p95 latency-p99]
      :or {total-processed 0 total-filtered 0 total-errors 0
           throughput 0.0 latency-p50 0.0 latency-p95 0.0 latency-p99 0.0}}]
  (map->PipelineMetrics
    {:pipeline-name pipeline-name
     :total-processed total-processed
     :total-filtered total-filtered
     :total-errors total-errors
     :throughput throughput
     :latency-p50 latency-p50
     :latency-p95 latency-p95
     :latency-p99 latency-p99}))
