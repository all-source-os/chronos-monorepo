(ns allsource.domain.entities.analytics
  "Domain entities for analytics and aggregations.
   Supports time-series analytics, statistical functions, funnel analysis, and cohorts."
  (:import [java.time Instant Duration]))

;; ============================================================================
;; Aggregation Types
;; ============================================================================

(def aggregation-functions
  "Valid aggregation functions."
  #{:count       ; Count events
    :sum         ; Sum numeric field
    :avg         ; Average numeric field
    :min         ; Minimum value
    :max         ; Maximum value
    :stddev      ; Standard deviation
    :variance    ; Variance
    :percentile  ; Percentile (P50, P95, P99)
    :distinct    ; Count distinct values
    :first       ; First value
    :last})      ; Last value

;; ============================================================================
;; Aggregation Entity
;; ============================================================================

(defrecord Aggregation
  [function      ; Aggregation function keyword
   field         ; Field to aggregate (or :* for count)
   alias         ; Result alias
   parameters])  ; Additional parameters (e.g., percentile value)

(defrecord AggregationResult
  [alias         ; Aggregation alias
   value         ; Computed value
   count         ; Number of events aggregated
   metadata])    ; Additional metadata

;; ============================================================================
;; Time-Series Entity
;; ============================================================================

(defrecord TimeSeriesConfig
  [interval      ; Time interval (:second, :minute, :hour, :day, :week, :month)
   field          ; Timestamp field (default :timestamp)
   aggregations   ; Vector of Aggregation entities
   fill-missing]) ; Strategy for missing data points (:zero, :null, :forward-fill)

(defrecord TimeSeriesPoint
  [timestamp     ; Point timestamp
   values        ; Map of alias => value
   metadata])    ; Additional metadata

(defrecord TimeSeries
  [name          ; Time series name
   interval       ; Time interval
   points         ; Vector of TimeSeriesPoint
   start-time     ; Series start time
   end-time       ; Series end time
   metadata])     ; Additional metadata

;; ============================================================================
;; Funnel Analysis Entity
;; ============================================================================

(defrecord FunnelStep
  [name          ; Step name
   predicate     ; Function (event) => boolean
   order])       ; Step order (1-indexed)

(defrecord FunnelConfig
  [name          ; Funnel name
   steps          ; Vector of FunnelStep
   time-window    ; Maximum time between steps (milliseconds)
   entity-id-fn]) ; Function to extract entity ID from event

(defrecord FunnelResult
  [funnel-name   ; Funnel name
   step-results   ; Map of step-name => {:entered N :completed N :drop-off N}
   conversion-rate ; Overall conversion rate (0.0 to 1.0)
   average-time   ; Average time to complete funnel
   metadata])     ; Additional metadata

;; ============================================================================
;; Cohort Analysis Entity
;; ============================================================================

(defrecord CohortConfig
  [name          ; Cohort name
   cohort-fn      ; Function (event) => cohort-key
   time-interval  ; Interval for cohort grouping (:day, :week, :month)
   metric-fn      ; Function (events) => metric value
   retention-periods]) ; Number of periods to track retention

(defrecord CohortResult
  [cohort-name   ; Cohort name
   cohorts        ; Map of cohort-key => {:size N :metrics [...]}
   retention-matrix ; 2D matrix of retention rates
   metadata])     ; Additional metadata

;; ============================================================================
;; Trend Analysis Entity
;; ============================================================================

(defrecord TrendConfig
  [metric-name   ; Metric being tracked
   time-interval  ; Interval for trend calculation
   smoothing      ; Smoothing algorithm (:none, :moving-average, :exponential)
   window-size])  ; Window size for smoothing

(defrecord TrendResult
  [metric-name   ; Metric name
   direction      ; Trend direction (:increasing, :decreasing, :stable)
   slope          ; Trend slope (rate of change)
   confidence     ; Confidence score (0.0 to 1.0)
   forecast       ; Forecasted values for next N periods
   metadata])     ; Additional metadata

;; ============================================================================
;; Anomaly Detection Entity
;; ============================================================================

(defrecord AnomalyConfig
  [metric-name   ; Metric to monitor
   algorithm      ; Detection algorithm (:zscore, :iqr, :mad)
   sensitivity    ; Sensitivity (1-10, higher = more sensitive)
   baseline-window]) ; Number of periods for baseline

(defrecord AnomalyResult
  [timestamp     ; Anomaly timestamp
   metric-name    ; Metric name
   actual-value   ; Actual value
   expected-value ; Expected value (from baseline)
   deviation      ; Deviation from expected
   severity       ; Severity score (0.0 to 1.0)
   metadata])     ; Additional metadata

;; ============================================================================
;; Validation Functions
;; ============================================================================

(defn valid-aggregation-function?
  "Check if aggregation function is valid."
  [function]
  (contains? aggregation-functions function))

(defn valid-aggregation?
  "Validate an aggregation."
  [aggregation]
  (and (instance? Aggregation aggregation)
       (valid-aggregation-function? (:function aggregation))
       (keyword? (:alias aggregation))))

(defn valid-time-series-interval?
  "Check if time series interval is valid."
  [interval]
  (contains? #{:second :minute :hour :day :week :month :year} interval))

(defn valid-funnel-step?
  "Validate a funnel step."
  [step]
  (and (instance? FunnelStep step)
       (keyword? (:name step))
       (fn? (:predicate step))
       (pos? (:order step))))

;; ============================================================================
;; Constructor Functions
;; ============================================================================

(defn make-aggregation
  "Create an aggregation.

   Parameters:
   - function: Aggregation function keyword
   - field: Field to aggregate
   - alias: Result alias
   - parameters: Optional parameters map

   Example:
   (make-aggregation :function :avg :field :amount :alias :avg-amount)"
  [& {:keys [function field alias parameters]
      :or {parameters {}}}]
  (map->Aggregation
    {:function function
     :field field
     :alias alias
     :parameters parameters}))

(defn make-time-series-config
  "Create time series configuration.

   Parameters:
   - interval: Time interval
   - field: Timestamp field (default :timestamp)
   - aggregations: Vector of aggregations
   - fill-missing: Strategy for missing data"
  [& {:keys [interval field aggregations fill-missing]
      :or {field :timestamp fill-missing :zero}}]
  (map->TimeSeriesConfig
    {:interval interval
     :field field
     :aggregations aggregations
     :fill-missing fill-missing}))

(defn make-funnel-step
  "Create a funnel step.

   Parameters:
   - name: Step name
   - predicate: Event predicate function
   - order: Step order"
  [& {:keys [name predicate order]}]
  (map->FunnelStep
    {:name name
     :predicate predicate
     :order order}))

(defn make-funnel-config
  "Create funnel configuration.

   Parameters:
   - name: Funnel name
   - steps: Vector of FunnelStep
   - time-window: Max time between steps (ms)
   - entity-id-fn: Function to extract entity ID"
  [& {:keys [name steps time-window entity-id-fn]
      :or {entity-id-fn :entity-id}}]
  (map->FunnelConfig
    {:name name
     :steps (vec (sort-by :order steps))
     :time-window time-window
     :entity-id-fn entity-id-fn}))

(defn make-cohort-config
  "Create cohort configuration."
  [& {:keys [name cohort-fn time-interval metric-fn retention-periods]
      :or {retention-periods 12}}]
  (map->CohortConfig
    {:name name
     :cohort-fn cohort-fn
     :time-interval time-interval
     :metric-fn metric-fn
     :retention-periods retention-periods}))

(defn make-trend-config
  "Create trend analysis configuration."
  [& {:keys [metric-name time-interval smoothing window-size]
      :or {smoothing :moving-average window-size 7}}]
  (map->TrendConfig
    {:metric-name metric-name
     :time-interval time-interval
     :smoothing smoothing
     :window-size window-size}))

(defn make-anomaly-config
  "Create anomaly detection configuration."
  [& {:keys [metric-name algorithm sensitivity baseline-window]
      :or {algorithm :zscore sensitivity 3 baseline-window 30}}]
  (map->AnomalyConfig
    {:metric-name metric-name
     :algorithm algorithm
     :sensitivity sensitivity
     :baseline-window baseline-window}))

;; ============================================================================
;; Aggregation Factories (Common Aggregations)
;; ============================================================================

(defn count-aggregation
  "Create count aggregation."
  [alias]
  (make-aggregation :function :count :field :* :alias alias))

(defn sum-aggregation
  "Create sum aggregation."
  [field alias]
  (make-aggregation :function :sum :field field :alias alias))

(defn avg-aggregation
  "Create average aggregation."
  [field alias]
  (make-aggregation :function :avg :field field :alias alias))

(defn min-aggregation
  "Create minimum aggregation."
  [field alias]
  (make-aggregation :function :min :field field :alias alias))

(defn max-aggregation
  "Create maximum aggregation."
  [field alias]
  (make-aggregation :function :max :field field :alias alias))

(defn percentile-aggregation
  "Create percentile aggregation.

   Parameters:
   - field: Field to aggregate
   - percentile: Percentile value (e.g., 95 for P95)
   - alias: Result alias"
  [field percentile alias]
  (make-aggregation
    :function :percentile
    :field field
    :alias alias
    :parameters {:percentile percentile}))

(defn distinct-count-aggregation
  "Create distinct count aggregation."
  [field alias]
  (make-aggregation :function :distinct :field field :alias alias))

;; ============================================================================
;; Helper Functions
;; ============================================================================

(defn interval-to-millis
  "Convert interval keyword to milliseconds."
  [interval]
  (case interval
    :second  1000
    :minute  60000
    :hour    3600000
    :day     86400000
    :week    604800000
    :month   2592000000  ; 30 days
    :year    31536000000 ; 365 days
    1000)) ; default: 1 second

(defn group-by-interval
  "Group events by time interval.

   Parameters:
   - events: Sequence of events
   - interval: Time interval keyword
   - timestamp-fn: Function to extract timestamp from event

   Returns: Map of interval-start-timestamp => [events]"
  [events interval timestamp-fn]
  (let [interval-ms (interval-to-millis interval)]
    (group-by (fn [event]
                (let [ts (timestamp-fn event)]
                  (* (quot ts interval-ms) interval-ms)))
              events)))

(defn create-time-series-points
  "Create time series points from grouped events.

   Parameters:
   - grouped-events: Map of timestamp => events
   - aggregations: Vector of Aggregation
   - fill-missing: Fill strategy

   Returns: Vector of TimeSeriesPoint"
  [grouped-events aggregations fill-missing]
  (for [[timestamp events] (sort-by key grouped-events)]
    (map->TimeSeriesPoint
      {:timestamp timestamp
       :values (into {} (map (fn [agg]
                               [(:alias agg)
                                (compute-aggregation agg events)])
                             aggregations))
       :metadata {:event-count (count events)}})))

(defn compute-aggregation
  "Compute aggregation value for events.
   This is a placeholder - actual implementation in use case layer."
  [aggregation events]
  ;; Placeholder - will be implemented in use case layer
  0)
