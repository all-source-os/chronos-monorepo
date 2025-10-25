(ns allsource.application.usecases.analytics-engine
  "Analytics engine implementation.
   Computes aggregations, time-series, funnels, cohorts, trends, and anomalies."
  (:require [allsource.domain.entities.analytics :as a]
            [clojure.set :as set])
  (:import [java.time Instant Duration]
           [java.lang Math]))

;; ============================================================================
;; Aggregation Computation
;; ============================================================================

(defn compute-count
  "Compute count aggregation."
  [events field]
  (count events))

(defn compute-sum
  "Compute sum aggregation."
  [events field]
  (reduce + 0 (map #(get-in % (if (vector? field) field [field]) 0) events)))

(defn compute-avg
  "Compute average aggregation."
  [events field]
  (if (empty? events)
    0.0
    (let [values (map #(get-in % (if (vector? field) field [field]) 0) events)
          total (reduce + 0 values)]
      (/ (double total) (count values)))))

(defn compute-min
  "Compute minimum aggregation."
  [events field]
  (if (empty? events)
    nil
    (apply min (map #(get-in % (if (vector? field) field [field]) 0) events))))

(defn compute-max
  "Compute maximum aggregation."
  [events field]
  (if (empty? events)
    nil
    (apply max (map #(get-in % (if (vector? field) field [field]) 0) events))))

(defn compute-stddev
  "Compute standard deviation."
  [events field]
  (if (empty? events)
    0.0
    (let [values (map #(get-in % (if (vector? field) field [field]) 0) events)
          mean (/ (reduce + 0 values) (count values))
          squared-diffs (map #(Math/pow (- % mean) 2) values)
          variance (/ (reduce + 0 squared-diffs) (count values))]
      (Math/sqrt variance))))

(defn compute-variance
  "Compute variance."
  [events field]
  (if (empty? events)
    0.0
    (let [stddev (compute-stddev events field)]
      (* stddev stddev))))

(defn compute-percentile
  "Compute percentile (P50, P95, P99, etc)."
  [events field percentile]
  (if (empty? events)
    nil
    (let [values (sort (map #(get-in % (if (vector? field) field [field]) 0) events))
          n (count values)
          index (int (* (/ percentile 100.0) (dec n)))]
      (nth values index))))

(defn compute-distinct
  "Compute distinct count."
  [events field]
  (count (distinct (map #(get-in % (if (vector? field) field [field])) events))))

(defn compute-first
  "Get first value."
  [events field]
  (when-let [first-event (first events)]
    (get-in first-event (if (vector? field) field [field]))))

(defn compute-last
  "Get last value."
  [events field]
  (when-let [last-event (last events)]
    (get-in last-event (if (vector? field) field [field]))))

(defn compute-aggregation
  "Compute aggregation value for events.

   Parameters:
   - aggregation: Aggregation entity
   - events: Sequence of events

   Returns: Computed value"
  [aggregation events]
  (let [function (:function aggregation)
        field (:field aggregation)
        parameters (:parameters aggregation)]
    (case function
      :count     (compute-count events field)
      :sum       (compute-sum events field)
      :avg       (compute-avg events field)
      :min       (compute-min events field)
      :max       (compute-max events field)
      :stddev    (compute-stddev events field)
      :variance  (compute-variance events field)
      :percentile (compute-percentile events field (:percentile parameters))
      :distinct  (compute-distinct events field)
      :first     (compute-first events field)
      :last      (compute-last events field)
      ;; Default
      0)))

;; ============================================================================
;; Time Series Analysis
;; ============================================================================

(defn compute-time-series
  "Compute time series from events.

   Parameters:
   - config: TimeSeriesConfig entity
   - events: Sequence of events
   - start-time: Series start time (Instant)
   - end-time: Series end time (Instant)

   Returns: TimeSeries entity"
  [config events start-time end-time]
  (let [interval (:interval config)
        field (:field config)
        aggregations (:aggregations config)
        fill-missing (:fill-missing config)
        ;; Group events by interval
        grouped (a/group-by-interval events interval
                                     #(get % field (:timestamp %)))
        ;; Create time series points
        points (a/create-time-series-points grouped aggregations fill-missing)
        ;; Fill missing intervals if needed
        filled-points (fill-missing-points points start-time end-time interval fill-missing)]
    (a/map->TimeSeries
      {:name (str (name interval) "-series")
       :interval interval
       :points filled-points
       :start-time start-time
       :end-time end-time
       :metadata {:total-points (count filled-points)}})))

(defn fill-missing-points
  "Fill missing time series points based on strategy."
  [points start-time end-time interval fill-strategy]
  (let [interval-ms (a/interval-to-millis interval)
        start-ts (.toEpochMilli start-time)
        end-ts (.toEpochMilli end-time)
        point-map (into {} (map (fn [p] [(:timestamp p) p]) points))]
    (vec
      (for [ts (range start-ts (+ end-ts interval-ms) interval-ms)]
        (or (get point-map ts)
            ;; Create missing point based on fill strategy
            (case fill-strategy
              :zero (a/map->TimeSeriesPoint
                      {:timestamp ts
                       :values {}
                       :metadata {:filled true :fill-strategy :zero}})
              :null nil
              :forward-fill (or (get point-map ts)
                                (a/map->TimeSeriesPoint
                                  {:timestamp ts
                                   :values {}
                                   :metadata {:filled true :fill-strategy :forward-fill}}))
              ;; Default: zero
              (a/map->TimeSeriesPoint
                {:timestamp ts
                 :values {}
                 :metadata {:filled true}})))))))

;; ============================================================================
;; Funnel Analysis
;; ============================================================================

(defn analyze-funnel
  "Analyze funnel conversion.

   Parameters:
   - config: FunnelConfig entity
   - events: Sequence of events

   Returns: FunnelResult entity"
  [config events]
  (let [steps (:steps config)
        time-window (:time-window config)
        entity-id-fn (:entity-id-fn config)
        ;; Group events by entity
        entity-events (group-by entity-id-fn events)
        ;; Analyze each entity's journey through funnel
        entity-results (for [[entity-id entity-evts] entity-events]
                         (analyze-entity-funnel entity-evts steps time-window))
        ;; Aggregate results
        step-results (aggregate-funnel-results entity-results steps)]
    (a/map->FunnelResult
      {:funnel-name (:name config)
       :step-results step-results
       :conversion-rate (calculate-conversion-rate step-results)
       :average-time (calculate-average-funnel-time entity-results)
       :metadata {:total-entities (count entity-events)}})))

(defn analyze-entity-funnel
  "Analyze single entity's journey through funnel."
  [entity-events steps time-window]
  (let [sorted-events (sort-by :timestamp entity-events)]
    (loop [remaining-steps steps
           remaining-events sorted-events
           completed-steps []
           start-time nil]
      (if (empty? remaining-steps)
        {:completed true
         :steps completed-steps
         :duration (when start-time
                     (- (:timestamp (last sorted-events)) start-time))}
        (let [step (first remaining-steps)
              predicate (:predicate step)
              ;; Find first event matching this step
              matching-event (first (filter predicate remaining-events))]
          (if matching-event
            (let [step-time (:timestamp matching-event)
                  step-start (or start-time step-time)]
              ;; Check time window
              (if (or (nil? time-window)
                      (< (- step-time step-start) time-window))
                (recur (rest remaining-steps)
                       (drop-while #(<= (:timestamp %) step-time) remaining-events)
                       (conj completed-steps step)
                       step-start)
                ;; Time window exceeded
                {:completed false
                 :steps completed-steps
                 :failed-at (:name step)}))
            ;; No matching event found
            {:completed false
             :steps completed-steps
             :failed-at (:name step)}))))))

(defn aggregate-funnel-results
  "Aggregate funnel results across all entities."
  [entity-results steps]
  (reduce (fn [acc step]
            (let [step-name (:name step)
                  entered (count (filter #(some #{step} (:steps %)) entity-results))
                  completed (count (filter #(and (:completed %)
                                                  (some #{step} (:steps %)))
                                           entity-results))
                  drop-off (- entered completed)]
              (assoc acc step-name
                     {:entered entered
                      :completed completed
                      :drop-off drop-off
                      :conversion-rate (if (pos? entered)
                                         (/ (double completed) entered)
                                         0.0)})))
          {}
          steps))

(defn calculate-conversion-rate
  "Calculate overall funnel conversion rate."
  [step-results]
  (let [first-step (first (vals step-results))
        last-step (last (vals step-results))]
    (if (and first-step last-step (pos? (:entered first-step)))
      (/ (double (:completed last-step)) (:entered first-step))
      0.0)))

(defn calculate-average-funnel-time
  "Calculate average time to complete funnel."
  [entity-results]
  (let [completed (filter :completed entity-results)
        durations (map :duration completed)]
    (if (empty? durations)
      0
      (/ (reduce + durations) (count durations)))))

;; ============================================================================
;; Trend Analysis
;; ============================================================================

(defn analyze-trend
  "Analyze trend in time series data.

   Parameters:
   - config: TrendConfig entity
   - data-points: Sequence of [timestamp value] pairs

   Returns: TrendResult entity"
  [config data-points]
  (let [smoothing (:smoothing config)
        window-size (:window-size config)
        ;; Apply smoothing
        smoothed (case smoothing
                   :moving-average (moving-average data-points window-size)
                   :exponential (exponential-smoothing data-points 0.3)
                   :none data-points
                   data-points)
        ;; Calculate trend
        slope (calculate-slope smoothed)
        direction (cond
                    (> slope 0.01) :increasing
                    (< slope -0.01) :decreasing
                    :else :stable)
        confidence (calculate-trend-confidence smoothed slope)]
    (a/map->TrendResult
      {:metric-name (:metric-name config)
       :direction direction
       :slope slope
       :confidence confidence
       :forecast (forecast-values smoothed 5)
       :metadata {:smoothing smoothing :window-size window-size}})))

(defn moving-average
  "Calculate moving average."
  [data-points window-size]
  (let [values (map second data-points)]
    (map-indexed (fn [i [ts v]]
                   [ts (if (< i window-size)
                         v
                         (/ (reduce + (take window-size (drop (- i window-size) values)))
                            window-size))])
                 data-points)))

(defn exponential-smoothing
  "Apply exponential smoothing."
  [data-points alpha]
  (reduce (fn [acc [ts value]]
            (let [prev-smoothed (if (empty? acc) value (second (last acc)))
                  smoothed (+ (* alpha value) (* (- 1 alpha) prev-smoothed))]
              (conj acc [ts smoothed])))
          []
          data-points))

(defn calculate-slope
  "Calculate slope using linear regression."
  [data-points]
  (if (< (count data-points) 2)
    0.0
    (let [n (count data-points)
          xs (range n)
          ys (map second data-points)
          mean-x (/ (reduce + xs) n)
          mean-y (/ (reduce + ys) n)
          numerator (reduce + (map #(* (- %1 mean-x) (- %2 mean-y)) xs ys))
          denominator (reduce + (map #(Math/pow (- % mean-x) 2) xs))]
      (if (zero? denominator)
        0.0
        (/ numerator denominator)))))

(defn calculate-trend-confidence
  "Calculate confidence score for trend."
  [data-points slope]
  ;; Simple confidence based on R-squared
  (if (< (count data-points) 2)
    0.0
    (let [ys (map second data-points)
          mean-y (/ (reduce + ys) (count ys))
          predicted (map-indexed #(+ mean-y (* slope %1)) ys)
          ss-tot (reduce + (map #(Math/pow (- % mean-y) 2) ys))
          ss-res (reduce + (map #(Math/pow (- %1 %2) 2) ys predicted))
          r-squared (if (zero? ss-tot) 0.0 (- 1.0 (/ ss-res ss-tot)))]
      (Math/max 0.0 (Math/min 1.0 r-squared)))))

(defn forecast-values
  "Forecast future values using linear regression."
  [data-points n-periods]
  (let [slope (calculate-slope data-points)
        last-value (second (last data-points))
        last-index (dec (count data-points))]
    (vec (for [i (range 1 (inc n-periods))]
           (+ last-value (* slope i))))))

;; ============================================================================
;; Anomaly Detection
;; ============================================================================

(defn detect-anomalies
  "Detect anomalies in time series data.

   Parameters:
   - config: AnomalyConfig entity
   - data-points: Sequence of [timestamp value] pairs

   Returns: Sequence of AnomalyResult entities"
  [config data-points]
  (let [algorithm (:algorithm config)
        sensitivity (:sensitivity config)
        baseline-window (:baseline-window config)]
    (case algorithm
      :zscore (detect-anomalies-zscore data-points sensitivity baseline-window config)
      :iqr (detect-anomalies-iqr data-points sensitivity config)
      :mad (detect-anomalies-mad data-points sensitivity config)
      [])))

(defn detect-anomalies-zscore
  "Detect anomalies using Z-score method."
  [data-points sensitivity baseline-window config]
  (let [values (map second data-points)
        baseline (take baseline-window values)
        mean (/ (reduce + baseline) (count baseline))
        stddev (Math/sqrt (/ (reduce + (map #(Math/pow (- % mean) 2) baseline))
                             (count baseline)))]
    (filter some?
            (map-indexed
              (fn [i [ts value]]
                (when (>= i baseline-window)
                  (let [z-score (if (zero? stddev) 0 (/ (- value mean) stddev))]
                    (when (> (Math/abs z-score) sensitivity)
                      (a/map->AnomalyResult
                        {:timestamp ts
                         :metric-name (:metric-name config)
                         :actual-value value
                         :expected-value mean
                         :deviation (- value mean)
                         :severity (/ (Math/abs z-score) sensitivity)
                         :metadata {:z-score z-score}})))))
              data-points))))

(defn detect-anomalies-iqr
  "Detect anomalies using IQR (Interquartile Range) method."
  [data-points sensitivity config]
  (let [values (sort (map second data-points))
        n (count values)
        q1 (nth values (int (* 0.25 n)))
        q3 (nth values (int (* 0.75 n)))
        iqr (- q3 q1)
        lower-bound (- q1 (* sensitivity iqr))
        upper-bound (+ q3 (* sensitivity iqr))]
    (filter some?
            (map (fn [[ts value]]
                   (when (or (< value lower-bound) (> value upper-bound))
                     (a/map->AnomalyResult
                       {:timestamp ts
                        :metric-name (:metric-name config)
                        :actual-value value
                        :expected-value (/ (+ q1 q3) 2)
                        :deviation (if (< value lower-bound)
                                     (- value lower-bound)
                                     (- value upper-bound))
                        :severity 0.8
                        :metadata {:method :iqr :q1 q1 :q3 q3}})))
                 data-points))))

(defn detect-anomalies-mad
  "Detect anomalies using MAD (Median Absolute Deviation) method."
  [data-points sensitivity config]
  (let [values (map second data-points)
        median (nth (sort values) (quot (count values) 2))
        abs-deviations (map #(Math/abs (- % median)) values)
        mad (nth (sort abs-deviations) (quot (count abs-deviations) 2))
        threshold (* sensitivity mad)]
    (filter some?
            (map (fn [[ts value]]
                   (when (> (Math/abs (- value median)) threshold)
                     (a/map->AnomalyResult
                       {:timestamp ts
                        :metric-name (:metric-name config)
                        :actual-value value
                        :expected-value median
                        :deviation (- value median)
                        :severity 0.7
                        :metadata {:method :mad :median median :mad mad}})))
                 data-points))))
