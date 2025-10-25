(ns allsource.application.usecases.pipeline-executor
  "Pipeline executor implementation.
   Executes event processing pipelines with operators."
  (:require [allsource.domain.entities.pipeline :as p]
            [clojure.core.async :as async :refer [>! <! go chan buffer dropping-buffer sliding-buffer timeout]]
            [clojure.tools.logging :as log])
  (:import [java.time Instant Duration]))

;; ============================================================================
;; Operator Application
;; ============================================================================

(defn apply-operator
  "Apply a single operator to a sequence of events.

   Parameters:
   - operator: PipelineOperator entity
   - events: Sequence of events

   Returns: Transformed sequence of events"
  [operator events]
  (let [op-type (:type operator)
        op-fn (:operator-fn operator)]
    (case op-type
      :filter    (filter op-fn events)
      :map       (map op-fn events)
      :flat-map  (mapcat op-fn events)
      :enrich    (map op-fn events)
      :batch     (let [batch-size (get-in operator [:config :batch-size])]
                   (partition-all batch-size events)
                   (map op-fn (partition-all batch-size events)))
      :window    (apply-window-operator operator events)
      :aggregate (apply-aggregate-operator operator events)
      :throttle  (apply-throttle-operator operator events)
      :deduplicate (apply-deduplicate-operator operator events)
      ;; Default: pass through
      events)))

(defn apply-operator-safe
  "Apply operator with error handling.

   Returns: {:status :success :result [...]} or {:status :error :error <msg>}"
  [operator events]
  (try
    {:status :success
     :result (apply-operator operator events)}
    (catch Exception e
      {:status :error
       :error (.getMessage e)
       :operator (:name operator)})))

;; ============================================================================
;; Window Operator Implementation
;; ============================================================================

(defn- tumbling-window
  "Create tumbling windows based on time or count.

   Parameters:
   - events: Sequence of events
   - size: Window size (milliseconds or count)

   Returns: Sequence of event batches (windows)"
  [events size]
  (let [sorted-events (sort-by :timestamp events)]
    (if (every? :timestamp sorted-events)
      ;; Time-based windows
      (let [first-ts (:timestamp (first sorted-events))]
        (loop [remaining sorted-events
               windows []
               window-start first-ts]
          (if (empty? remaining)
            windows
            (let [window-end (+ window-start size)
                  [window-events rest-events] (split-with
                                                #(< (:timestamp %) window-end)
                                                remaining)]
              (recur rest-events
                     (conj windows window-events)
                     window-end)))))
      ;; Count-based windows
      (partition-all size sorted-events))))

(defn- sliding-window
  "Create sliding windows.

   Parameters:
   - events: Sequence of events
   - size: Window size (milliseconds)
   - slide: Slide interval (milliseconds)

   Returns: Sequence of event batches (windows)"
  [events size slide]
  (let [sorted-events (sort-by :timestamp events)
        first-ts (:timestamp (first sorted-events))
        last-ts (:timestamp (last sorted-events))]
    (for [window-start (range first-ts (+ last-ts size) slide)]
      (let [window-end (+ window-start size)]
        (filter #(and (>= (:timestamp %) window-start)
                      (< (:timestamp %) window-end))
                sorted-events)))))

(defn apply-window-operator
  "Apply window operator to events.

   Parameters:
   - operator: Window operator with config
   - events: Sequence of events

   Returns: Sequence of aggregated window results"
  [operator events]
  (let [window-config (get-in operator [:config :window])
        window-type (:type window-config)
        window-size (:size window-config)
        aggregate-fn (:operator-fn operator)]
    (case window-type
      :tumbling (map aggregate-fn (tumbling-window events window-size))
      :sliding  (let [slide (or (:slide window-config) (quot window-size 2))]
                  (map aggregate-fn (sliding-window events window-size slide)))
      :session  (throw (ex-info "Session windows not yet implemented" {}))
      ;; Default: tumbling
      (map aggregate-fn (tumbling-window events window-size)))))

;; ============================================================================
;; Other Operator Implementations
;; ============================================================================

(defn apply-aggregate-operator
  "Apply aggregation operator."
  [operator events]
  (let [aggregate-fn (:operator-fn operator)]
    [(aggregate-fn events)]))

(defn apply-throttle-operator
  "Apply throttle operator (rate limiting)."
  [operator events]
  (let [rate-limit (get-in operator [:config :rate-limit] 100) ; events per second
        interval-ms (/ 1000 rate-limit)]
    (doseq [event events]
      (Thread/sleep (long interval-ms)))
    events))

(defn apply-deduplicate-operator
  "Apply deduplication operator."
  [operator events]
  (let [key-fn (or (get-in operator [:config :key-fn]) :entity-id)]
    (vals (reduce (fn [acc event]
                    (let [k (key-fn event)]
                      (if (contains? acc k)
                        acc
                        (assoc acc k event))))
                  {}
                  events))))

;; ============================================================================
;; Pipeline Execution
;; ============================================================================

(defn execute-pipeline
  "Execute a complete pipeline.

   Parameters:
   - pipeline: Pipeline entity
   - events: Sequence of events

   Returns: Transformed sequence of events"
  [pipeline events]
  (let [operators (p/get-operators pipeline)]
    (reduce (fn [evts operator]
              (let [result (apply-operator operator evts)]
                ;; Realize lazy sequences to avoid stack overflow
                (if (or (seq? result) (instance? clojure.lang.LazySeq result))
                  (doall result)
                  result)))
            events
            operators)))

(defn execute-pipeline-safe
  "Execute pipeline with error handling.

   Returns: {:status :success :result [...]} or {:status :error :error <msg>}"
  [pipeline events]
  (try
    {:status :success
     :result (execute-pipeline pipeline events)}
    (catch Exception e
      {:status :error
       :error (.getMessage e)
       :pipeline (p/get-name pipeline)})))

;; ============================================================================
;; Async Pipeline Execution
;; ============================================================================

(defn execute-pipeline-async
  "Execute pipeline asynchronously.

   Parameters:
   - pipeline: Pipeline entity
   - events: Sequence of events
   - callback: Function (result) => void

   Returns: nil (result delivered to callback)"
  [pipeline events callback]
  (future
    (try
      (let [result (execute-pipeline pipeline events)]
        (callback result))
      (catch Exception e
        (callback {:error (.getMessage e)})))))

;; ============================================================================
;; Pipeline Executor Record
;; ============================================================================

(defrecord PipelineExecutor
  [config                    ; Executor configuration
   backpressure-config       ; Backpressure configuration
   metrics-atom              ; Atom for metrics collection
   parallel                  ; Parallel execution flag
   parallelism])             ; Degree of parallelism

(defn create-executor
  "Create a new pipeline executor.

   Options:
   - :backpressure-config - BackpressureConfig entity
   - :parallel - Enable parallel execution (default false)
   - :parallelism - Degree of parallelism (default 4)
   - :collect-metrics - Collect metrics (default true)

   Returns: PipelineExecutor instance"
  [& {:keys [backpressure-config parallel parallelism collect-metrics]
      :or {parallel false parallelism 4 collect-metrics true}}]
  (map->PipelineExecutor
    {:config {:collect-metrics collect-metrics}
     :backpressure-config backpressure-config
     :metrics-atom (atom {})
     :parallel parallel
     :parallelism parallelism}))

;; ============================================================================
;; Metrics Collection
;; ============================================================================

(defn- record-metrics
  "Record metrics for pipeline execution."
  [metrics-atom pipeline-name total-input total-output duration-ms]
  (let [filtered-count (- total-input total-output)
        throughput (if (pos? duration-ms)
                     (/ (* total-input 1000.0) duration-ms)
                     0.0)]
    (swap! metrics-atom assoc pipeline-name
           {:total-processed total-input
            :total-filtered filtered-count
            :total-output total-output
            :throughput throughput
            :duration-ms duration-ms})))

(defn execute-with-metrics
  "Execute pipeline and collect metrics.

   Parameters:
   - executor: PipelineExecutor instance
   - pipeline: Pipeline entity
   - events: Sequence of events

   Returns: {:result [...] :metrics {...}}"
  [executor pipeline events]
  (let [start-time (System/currentTimeMillis)
        input-count (count events)
        result (execute-pipeline pipeline events)
        output-count (count result)
        duration (- (System/currentTimeMillis) start-time)
        pipeline-name (p/get-name pipeline)]
    (record-metrics (:metrics-atom executor) pipeline-name
                    input-count output-count duration)
    {:result result
     :metrics (p/make-metrics
                :pipeline-name pipeline-name
                :total-processed input-count
                :total-filtered (- input-count output-count)
                :total-errors 0
                :throughput (if (pos? duration)
                              (/ (* input-count 1000.0) duration)
                              0.0))
     :operator-metrics (collect-operator-metrics executor pipeline events)}))

(defn collect-operator-metrics
  "Collect metrics for each operator in the pipeline."
  [executor pipeline events]
  (let [operators (p/get-operators pipeline)]
    (reduce (fn [metrics operator]
              (let [op-name (:name operator)
                    start-time (System/currentTimeMillis)
                    input-events (if (empty? metrics)
                                   events
                                   (:output (last (vals metrics))))
                    output-events (try
                                    (doall (apply-operator operator input-events))
                                    (catch Exception e
                                      []))
                    duration (- (System/currentTimeMillis) start-time)]
                (assoc metrics op-name
                       {:input-count (count input-events)
                        :output-count (count output-events)
                        :duration-ms duration
                        :output output-events})))
            {}
            operators)))

;; ============================================================================
;; Backpressure Handling
;; ============================================================================

(defn execute-with-backpressure
  "Execute pipeline with backpressure handling.

   Parameters:
   - executor: PipelineExecutor instance
   - pipeline: Pipeline entity
   - events: Sequence of events

   Returns: Sequence of processed events"
  [executor pipeline events]
  (if-let [bp-config (:backpressure-config executor)]
    (let [strategy (:strategy bp-config)
          buffer-size (:buffer-size bp-config)
          timeout-ms (:timeout-ms bp-config)]
      (case strategy
        :drop    (let [in-chan (chan (dropping-buffer buffer-size))
                       out-chan (chan buffer-size)]
                   (async/onto-chan!! in-chan events false)
                   (go
                     (loop []
                       (when-let [event (<! in-chan)]
                         (let [result (execute-pipeline pipeline [event])]
                           (doseq [r result]
                             (>! out-chan r)))
                         (recur))))
                   (async/<!! (async/into [] out-chan)))

        :buffer  (let [in-chan (chan (buffer buffer-size))
                       out-chan (chan buffer-size)]
                   (async/onto-chan!! in-chan events false)
                   (go
                     (loop []
                       (when-let [event (<! in-chan)]
                         (let [result (execute-pipeline pipeline [event])]
                           (doseq [r result]
                             (>! out-chan r)))
                         (recur))))
                   (async/<!! (async/into [] out-chan)))

        :block   (let [in-chan (chan buffer-size)
                       out-chan (chan buffer-size)]
                   (async/onto-chan!! in-chan events false)
                   (go
                     (loop []
                       (when-let [event (<! in-chan)]
                         (let [result (execute-pipeline pipeline [event])]
                           (doseq [r result]
                             (>! out-chan r)))
                         (recur))))
                   (let [timeout-chan (timeout timeout-ms)]
                     (async/alt!!
                       (async/into [] out-chan) ([v] v)
                       timeout-chan [])))

        ;; Default: no backpressure
        (execute-pipeline pipeline events)))
    ;; No backpressure config
    (execute-pipeline pipeline events)))

;; ============================================================================
;; Parallel Execution
;; ============================================================================

(defn execute-pipeline-parallel
  "Execute pipeline in parallel using pmap.

   Parameters:
   - executor: PipelineExecutor instance
   - pipeline: Pipeline entity
   - events: Sequence of events

   Returns: Sequence of processed events"
  [executor pipeline events]
  (if (:parallel executor)
    (let [parallelism (or (:parallelism executor) 4)
          ;; Partition events for parallel processing
          partitions (partition-all (/ (count events) parallelism) events)
          ;; Process each partition in parallel
          results (pmap #(execute-pipeline pipeline %) partitions)]
      ;; Flatten results
      (apply concat results))
    ;; Not parallel, use regular execution
    (execute-pipeline pipeline events)))

;; ============================================================================
;; Helper Functions
;; ============================================================================

(defn get-executor-metrics
  "Get all metrics collected by the executor."
  [executor]
  @(:metrics-atom executor))

(defn reset-executor-metrics
  "Reset executor metrics."
  [executor]
  (reset! (:metrics-atom executor) {}))

(defn get-pipeline-metrics
  "Get metrics for a specific pipeline."
  [executor pipeline-name]
  (get @(:metrics-atom executor) pipeline-name))
