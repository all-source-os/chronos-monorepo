(ns allsource.application.usecases.integration-tools
  "Integration tools implementation.
   Event replay, validation, and schema migration."
  (:require [allsource.domain.entities.integration :as i]
            [clojure.tools.logging :as log])
  (:import [java.time Instant Duration]))

;; ============================================================================
;; Event Replay
;; ============================================================================

(defn replay-events
  "Replay events according to configuration.

   Parameters:
   - config: ReplayConfig entity
   - fetch-events-fn: Function (start-time, end-time) => events
   - handler-fn: Function (event) => void

   Returns: ReplayResult entity"
  [config fetch-events-fn handler-fn]
  (let [start-time (Instant/now)
        events (fetch-events-fn (:start-time config) (:end-time config))
        filtered-events (if-let [filter-fn (:filter-fn config)]
                          (filter filter-fn events)
                          events)
        batch-size (:batch-size config)
        speed (:speed config)
        parallel (:parallel config)]
    (try
      (let [events-count (if parallel
                           (replay-parallel filtered-events handler-fn batch-size speed)
                           (replay-sequential filtered-events handler-fn batch-size speed))
            end-time (Instant/now)
            duration (.toMillis (Duration/between start-time end-time))]
        (i/map->ReplayResult
          {:replay-name (:name config)
           :status :completed
           :events-replayed events-count
           :start-time start-time
           :end-time end-time
           :duration-ms duration
           :errors []
           :metadata {:speed speed :parallel parallel}}))
      (catch Exception e
        (i/map->ReplayResult
          {:replay-name (:name config)
           :status :failed
           :events-replayed 0
           :start-time start-time
           :end-time (Instant/now)
           :duration-ms 0
           :errors [(.getMessage e)]
           :metadata {:error (.getMessage e)}})))))

(defn replay-sequential
  "Replay events sequentially."
  [events handler-fn batch-size speed]
  (let [batches (partition-all batch-size events)]
    (reduce (fn [count batch]
              (doseq [event batch]
                (try
                  (handler-fn event)
                  (catch Exception e
                    (log/error e "Error replaying event" event))))
              ;; Apply speed throttling
              (when (and speed (pos? speed))
                (let [delay-ms (/ (* batch-size 1000) speed)]
                  (Thread/sleep (long delay-ms))))
              (+ count (count batch)))
            0
            batches)))

(defn replay-parallel
  "Replay events in parallel."
  [events handler-fn batch-size speed]
  (let [batches (partition-all batch-size events)
        results (pmap (fn [batch]
                        (doseq [event batch]
                          (try
                            (handler-fn event)
                            (catch Exception e
                              (log/error e "Error replaying event" event))))
                        (count batch))
                      batches)]
    (reduce + results)))

(defn replay-to-projection
  "Replay events to rebuild a projection.

   Parameters:
   - config: ReplayConfig entity
   - fetch-events-fn: Function to fetch events
   - projection-executor: Projection executor
   - projection-name: Name of projection to rebuild

   Returns: ReplayResult entity"
  [config fetch-events-fn projection-executor projection-name]
  (replay-events config
                 fetch-events-fn
                 (fn [event]
                   ;; Process event through projection
                   (require '[allsource.application.usecases.projection-executor :as exec])
                   ((resolve 'exec/process-event) projection-executor projection-name event))))

;; ============================================================================
;; Event Validation
;; ============================================================================

(defn validate-events
  "Validate events according to configuration.

   Parameters:
   - config: ValidationConfig entity
   - events: Sequence of events

   Returns: ValidationResult entity"
  [config events]
  (let [rules (filter :enabled (:rules config))
        fail-fast (:fail-fast config)
        collect-all (:collect-all config)
        max-errors (:max-errors config)]
    (loop [remaining-events events
           valid-count 0
           invalid-count 0
           errors []
           warnings []]
      (if (or (empty? remaining-events)
              (and fail-fast (not (empty? errors)))
              (>= (count errors) max-errors))
        (i/map->ValidationResult
          {:config-name (:name config)
           :total-events (count events)
           :valid-events valid-count
           :invalid-events invalid-count
           :errors errors
           :warnings warnings
           :status (cond
                     (not (empty? errors)) :failed
                     (not (empty? warnings)) :warnings
                     :else :passed)})
        (let [event (first remaining-events)
              validation-results (validate-event event rules)]
          (if (empty? validation-results)
            ;; Valid event
            (recur (rest remaining-events)
                   (inc valid-count)
                   invalid-count
                   errors
                   warnings)
            ;; Invalid event
            (let [new-errors (filter #(= :error (:severity %)) validation-results)
                  new-warnings (filter #(= :warning (:severity %)) validation-results)]
              (recur (rest remaining-events)
                     valid-count
                     (if (empty? new-errors) invalid-count (inc invalid-count))
                     (if collect-all
                       (concat errors new-errors)
                       errors)
                     (if collect-all
                       (concat warnings new-warnings)
                       warnings)))))))))

(defn validate-event
  "Validate a single event against rules.

   Parameters:
   - event: Event to validate
   - rules: Vector of ValidationRule

   Returns: Vector of ValidationError (empty if valid)"
  [event rules]
  (reduce (fn [errors rule]
            (if ((:predicate rule) event)
              errors
              (conj errors
                    (i/map->ValidationError
                      {:rule-name (:name rule)
                       :event event
                       :severity (:severity rule)
                       :message ((:message-fn rule) event)
                       :timestamp (Instant/now)}))))
          []
          rules))

(defn validate-schema
  "Validate event against schema.

   Parameters:
   - event: Event to validate
   - schema: Schema entity

   Returns: Vector of validation errors"
  [event schema]
  (let [required (:required schema)
        fields (:fields schema)
        validators (:validators schema)]
    (concat
      ;; Check required fields
      (reduce (fn [errors field]
                (if (contains? event field)
                  errors
                  (conj errors
                        (i/map->ValidationError
                          {:rule-name :required-field
                           :event event
                           :severity :error
                           :message (str "Missing required field: " field)
                           :timestamp (Instant/now)}))))
              []
              required)
      ;; Run custom validators
      (mapcat (fn [validator-fn]
                (try
                  (if (validator-fn event)
                    []
                    [(i/map->ValidationError
                       {:rule-name :custom-validator
                        :event event
                        :severity :error
                        :message "Custom validation failed"
                        :timestamp (Instant/now)})])
                  (catch Exception e
                    [(i/map->ValidationError
                       {:rule-name :custom-validator
                        :event event
                        :severity :error
                        :message (.getMessage e)
                        :timestamp (Instant/now)})])))
              validators))))

;; ============================================================================
;; Schema Migration
;; ============================================================================

(defn migrate-events
  "Migrate events according to configuration.

   Parameters:
   - config: MigrationConfig entity
   - events: Sequence of events
   - current-version: Current schema version
   - target-version: Target schema version

   Returns: MigrationResult entity"
  [config events current-version target-version]
  (let [steps (:steps config)
        relevant-steps (filter (fn [step]
                                 (and (>= (:from-version step) current-version)
                                      (<= (:to-version step) target-version)))
                               steps)
        sorted-steps (sort-by :from-version relevant-steps)]
    (try
      (let [migrated-events (migrate-events-through-steps events sorted-steps)
            validate-after (:validate-after config)
            validation-errors (if validate-after
                                (validate-migrated-events migrated-events)
                                [])
            dry-run (:dry-run config)]
        (i/map->MigrationResult
          {:schema-name (:schema-name config)
           :from-version current-version
           :to-version target-version
           :events-migrated (if dry-run 0 (count migrated-events))
           :events-failed (count validation-errors)
           :errors validation-errors
           :status (if (empty? validation-errors)
                     :completed
                     (if (< (count validation-errors) (count events))
                       :partial
                       :failed))}))
      (catch Exception e
        (i/map->MigrationResult
          {:schema-name (:schema-name config)
           :from-version current-version
           :to-version target-version
           :events-migrated 0
           :events-failed (count events)
           :errors [(.getMessage e)]
           :status :failed})))))

(defn migrate-events-through-steps
  "Migrate events through a series of migration steps."
  [events steps]
  (reduce (fn [current-events step]
            (let [transform-fn (:transform-fn step)]
              (map transform-fn current-events)))
          events
          steps))

(defn migrate-event
  "Migrate a single event from one version to another.

   Parameters:
   - event: Event to migrate
   - from-version: Current version
   - to-version: Target version
   - steps: Vector of MigrationStep

   Returns: Migrated event"
  [event from-version to-version steps]
  (let [path (find-migration-path from-version to-version steps)]
    (reduce (fn [current-event step]
              ((:transform-fn step) current-event))
            event
            path)))

(defn find-migration-path
  "Find path of migration steps from source to target version."
  [from-version to-version steps]
  (let [sorted-steps (sort-by :from-version steps)
        valid-steps (filter (fn [step]
                              (and (>= (:from-version step) from-version)
                                   (<= (:to-version step) to-version)))
                            sorted-steps)]
    valid-steps))

(defn validate-migrated-events
  "Validate events after migration."
  [events]
  ;; Placeholder - could add schema validation here
  [])

;; ============================================================================
;; Rollback Migration
;; ============================================================================

(defn rollback-migration
  "Rollback a migration if all steps are reversible.

   Parameters:
   - config: MigrationConfig entity
   - events: Migrated events
   - from-version: Source version
   - to-version: Target version

   Returns: MigrationResult entity with rollback status"
  [config events from-version to-version]
  (let [steps (:steps config)
        relevant-steps (filter (fn [step]
                                 (and (>= (:from-version step) to-version)
                                      (<= (:to-version step) from-version)))
                               steps)
        sorted-steps (reverse (sort-by :from-version relevant-steps))]
    (if (every? :reversible sorted-steps)
      (try
        (let [rolled-back (reduce (fn [current-events step]
                                    (map (:reverse-fn step) current-events))
                                  events
                                  sorted-steps)]
          (i/map->MigrationResult
            {:schema-name (:schema-name config)
             :from-version from-version
             :to-version to-version
             :events-migrated (count rolled-back)
             :events-failed 0
             :errors []
             :status :completed}))
        (catch Exception e
          (i/map->MigrationResult
            {:schema-name (:schema-name config)
             :from-version from-version
             :to-version to-version
             :events-migrated 0
             :events-failed (count events)
             :errors [(.getMessage e)]
             :status :failed})))
      (i/map->MigrationResult
        {:schema-name (:schema-name config)
         :from-version from-version
         :to-version to-version
         :events-migrated 0
         :events-failed (count events)
         :errors ["Not all migration steps are reversible"]
         :status :failed}))))

;; ============================================================================
;; Data Quality Metrics
;; ============================================================================

(defn calculate-data-quality-metrics
  "Calculate data quality metrics for events.

   Parameters:
   - events: Sequence of events
   - schema: Schema entity
   - validation-config: ValidationConfig entity

   Returns: DataQualityMetrics entity"
  [events schema validation-config]
  (let [total (count events)
        ;; Completeness: percentage with all required fields
        required-fields (:required schema)
        complete-events (filter (fn [event]
                                  (every? #(contains? event %) required-fields))
                                events)
        completeness (* 100.0 (/ (count complete-events) total))
        ;; Correctness: percentage passing validation
        validation-result (validate-events validation-config events)
        correctness (* 100.0 (/ (:valid-events validation-result) total))
        ;; Uniqueness: percentage of unique events (by entity-id + event-type)
        unique-events (distinct (map (fn [e] [(:entity-id e) (:event-type e)]) events))
        uniqueness (* 100.0 (/ (count unique-events) total))
        ;; Consistency: check for consistent field types
        consistency (calculate-consistency events)
        ;; Timeliness: based on event timestamps
        timeliness (calculate-timeliness events)]
    (i/map->DataQualityMetrics
      {:completeness completeness
       :correctness correctness
       :consistency consistency
       :timeliness timeliness
       :uniqueness uniqueness
       :total-events total
       :metadata {:schema-version (:version schema)}})))

(defn calculate-consistency
  "Calculate consistency score (0-100)."
  [events]
  ;; Simple consistency check: do fields have consistent types?
  (let [field-types (reduce (fn [acc event]
                              (reduce (fn [acc2 [k v]]
                                        (update acc2 k (fnil conj #{}) (type v)))
                                      acc
                                      event))
                            {}
                            events)
        inconsistent-fields (filter (fn [[k types]] (> (count types) 1)) field-types)
        consistency-score (if (empty? field-types)
                            100.0
                            (* 100.0 (- 1.0 (/ (count inconsistent-fields)
                                               (count field-types)))))]
    consistency-score))

(defn calculate-timeliness
  "Calculate timeliness score (0-100) based on event timestamps."
  [events]
  ;; Check if events have recent timestamps
  (let [now (.toEpochMilli (Instant/now))
        one-day-ms 86400000
        recent-events (filter (fn [event]
                                (when-let [ts (:timestamp event)]
                                  (< (- now ts) one-day-ms)))
                              events)
        timeliness (* 100.0 (/ (count recent-events) (count events)))]
    timeliness))
