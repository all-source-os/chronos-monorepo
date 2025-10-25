(ns allsource.domain.entities.integration
  "Domain entities for integration tools.
   Supports event replay, validation, and schema migration."
  (:import [java.time Instant]))

;; ============================================================================
;; Replay Configuration
;; ============================================================================

(defrecord ReplayConfig
  [name           ; Replay job name
   start-time     ; Replay start time (Instant or timestamp)
   end-time       ; Replay end time (Instant or timestamp)
   filter-fn      ; Optional filter function
   speed          ; Replay speed multiplier (1.0 = real-time, 0 = max speed)
   target         ; Target (projection, pipeline, or custom handler)
   batch-size     ; Batch size for replay
   parallel])     ; Parallel replay flag

(defrecord ReplayResult
  [replay-name   ; Replay job name
   status         ; Status (:completed, :failed, :in-progress)
   events-replayed ; Number of events replayed
   start-time     ; Replay start time
   end-time       ; Replay end time
   duration-ms    ; Duration in milliseconds
   errors         ; Vector of errors encountered
   metadata])     ; Additional metadata

;; ============================================================================
;; Validation Configuration
;; ============================================================================

(defrecord ValidationRule
  [name          ; Rule name
   predicate     ; Validation predicate function (event) => boolean
   severity      ; Severity (:error, :warning, :info)
   message-fn    ; Function (event) => error message
   enabled])     ; Whether rule is enabled

(defrecord ValidationConfig
  [name          ; Validation config name
   rules          ; Vector of ValidationRule
   fail-fast      ; Stop on first error
   collect-all    ; Collect all validation errors
   max-errors])   ; Maximum errors to collect

(defrecord ValidationError
  [rule-name     ; Rule that failed
   event          ; Event that failed validation
   severity       ; Error severity
   message        ; Error message
   timestamp])    ; When error occurred

(defrecord ValidationResult
  [config-name   ; Validation config name
   total-events   ; Total events validated
   valid-events   ; Number of valid events
   invalid-events ; Number of invalid events
   errors         ; Vector of ValidationError
   warnings       ; Vector of ValidationError (warnings)
   status])       ; Overall status (:passed, :failed, :warnings)

;; ============================================================================
;; Schema Migration
;; ============================================================================

(defrecord Schema
  [name          ; Schema name
   version        ; Schema version
   fields         ; Map of field-name => field-spec
   required       ; Set of required fields
   validators     ; Vector of validator functions
   metadata])     ; Additional metadata

(defrecord MigrationStep
  [from-version  ; Source schema version
   to-version    ; Target schema version
   transform-fn   ; Transformation function (event) => migrated-event
   reversible     ; Whether migration is reversible
   reverse-fn])   ; Reverse transformation function

(defrecord MigrationConfig
  [schema-name   ; Schema name
   steps          ; Vector of MigrationStep (ordered)
   validate-after ; Validate events after migration
   dry-run])      ; Dry run mode (don't persist changes)

(defrecord MigrationResult
  [schema-name   ; Schema name
   from-version   ; Source version
   to-version     ; Target version
   events-migrated ; Number of events migrated
   events-failed  ; Number of events that failed migration
   errors         ; Vector of errors
   status])       ; Status (:completed, :failed, :partial)

;; ============================================================================
;; Data Quality Metrics
;; ============================================================================

(defrecord DataQualityMetrics
  [completeness  ; Percentage of events with all required fields (0-100)
   correctness   ; Percentage of events passing validation (0-100)
   consistency   ; Consistency score (0-100)
   timeliness    ; Timeliness score based on event timestamps (0-100)
   uniqueness    ; Percentage of unique events (0-100)
   total-events  ; Total events analyzed
   metadata])    ; Additional metrics

;; ============================================================================
;; Validation Functions
;; ============================================================================

(defn valid-replay-speed?
  "Check if replay speed is valid."
  [speed]
  (and (number? speed) (>= speed 0)))

(defn valid-validation-severity?
  "Check if validation severity is valid."
  [severity]
  (contains? #{:error :warning :info} severity))

(defn valid-validation-rule?
  "Validate a validation rule."
  [rule]
  (and (instance? ValidationRule rule)
       (keyword? (:name rule))
       (fn? (:predicate rule))
       (valid-validation-severity? (:severity rule))))

(defn valid-migration-step?
  "Validate a migration step."
  [step]
  (and (instance? MigrationStep step)
       (pos? (:from-version step))
       (pos? (:to-version step))
       (fn? (:transform-fn step))))

;; ============================================================================
;; Constructor Functions
;; ============================================================================

(defn make-replay-config
  "Create replay configuration.

   Parameters:
   - name: Replay job name
   - start-time: Start time (Instant or timestamp)
   - end-time: End time (Instant or timestamp)
   - filter-fn: Optional filter function
   - speed: Replay speed (default 0 = max speed)
   - target: Target for replay
   - batch-size: Batch size (default 1000)
   - parallel: Parallel replay (default false)

   Example:
   (make-replay-config
     :name :rebuild-projection
     :start-time (Instant/parse \"2025-01-01T00:00:00Z\")
     :end-time (Instant/now)
     :target :user-stats-projection)"
  [& {:keys [name start-time end-time filter-fn speed target batch-size parallel]
      :or {speed 0 batch-size 1000 parallel false}}]
  (map->ReplayConfig
    {:name name
     :start-time start-time
     :end-time end-time
     :filter-fn filter-fn
     :speed speed
     :target target
     :batch-size batch-size
     :parallel parallel}))

(defn make-validation-rule
  "Create validation rule.

   Parameters:
   - name: Rule name
   - predicate: Validation predicate
   - severity: Severity (:error, :warning, :info)
   - message-fn: Function to generate error message
   - enabled: Whether rule is enabled (default true)

   Example:
   (make-validation-rule
     :name :has-timestamp
     :predicate (fn [e] (some? (:timestamp e)))
     :severity :error
     :message-fn (fn [e] \"Event missing timestamp\"))"
  [& {:keys [name predicate severity message-fn enabled]
      :or {enabled true}}]
  (map->ValidationRule
    {:name name
     :predicate predicate
     :severity severity
     :message-fn message-fn
     :enabled enabled}))

(defn make-validation-config
  "Create validation configuration."
  [& {:keys [name rules fail-fast collect-all max-errors]
      :or {fail-fast false collect-all true max-errors 1000}}]
  (map->ValidationConfig
    {:name name
     :rules rules
     :fail-fast fail-fast
     :collect-all collect-all
     :max-errors max-errors}))

(defn make-schema
  "Create schema definition.

   Parameters:
   - name: Schema name
   - version: Schema version
   - fields: Map of field specifications
   - required: Set of required fields
   - validators: Vector of validator functions"
  [& {:keys [name version fields required validators metadata]
      :or {metadata {}}}]
  (map->Schema
    {:name name
     :version version
     :fields fields
     :required (set required)
     :validators (vec validators)
     :metadata metadata}))

(defn make-migration-step
  "Create migration step.

   Parameters:
   - from-version: Source version
   - to-version: Target version
   - transform-fn: Transformation function
   - reversible: Whether reversible (default false)
   - reverse-fn: Reverse transformation function"
  [& {:keys [from-version to-version transform-fn reversible reverse-fn]
      :or {reversible false}}]
  (map->MigrationStep
    {:from-version from-version
     :to-version to-version
     :transform-fn transform-fn
     :reversible reversible
     :reverse-fn reverse-fn}))

(defn make-migration-config
  "Create migration configuration."
  [& {:keys [schema-name steps validate-after dry-run]
      :or {validate-after true dry-run false}}]
  (map->MigrationConfig
    {:schema-name schema-name
     :steps (vec (sort-by :from-version steps))
     :validate-after validate-after
     :dry-run dry-run}))

;; ============================================================================
;; Common Validation Rules
;; ============================================================================

(defn required-field-rule
  "Create validation rule for required field."
  [field-name severity]
  (make-validation-rule
    :name (keyword (str "required-" (name field-name)))
    :predicate (fn [event]
                 (some? (get event field-name)))
    :severity severity
    :message-fn (fn [event]
                  (str "Missing required field: " field-name))))

(defn field-type-rule
  "Create validation rule for field type."
  [field-name expected-type severity]
  (make-validation-rule
    :name (keyword (str (name field-name) "-type"))
    :predicate (fn [event]
                 (let [value (get event field-name)]
                   (or (nil? value)
                       (case expected-type
                         :string (string? value)
                         :number (number? value)
                         :boolean (boolean? value)
                         :map (map? value)
                         :vector (vector? value)
                         true))))
    :severity severity
    :message-fn (fn [event]
                  (str "Field " field-name " must be of type " expected-type))))

(defn field-range-rule
  "Create validation rule for numeric field range."
  [field-name min-value max-value severity]
  (make-validation-rule
    :name (keyword (str (name field-name) "-range"))
    :predicate (fn [event]
                 (let [value (get event field-name)]
                   (or (nil? value)
                       (and (number? value)
                            (>= value min-value)
                            (<= value max-value)))))
    :severity severity
    :message-fn (fn [event]
                  (str "Field " field-name " must be between "
                       min-value " and " max-value))))

(defn event-type-rule
  "Create validation rule for event type."
  [allowed-types severity]
  (let [allowed-set (set allowed-types)]
    (make-validation-rule
      :name :valid-event-type
      :predicate (fn [event]
                   (contains? allowed-set (:event-type event)))
      :severity severity
      :message-fn (fn [event]
                    (str "Invalid event type: " (:event-type event)
                         ". Allowed: " (pr-str allowed-types))))))

;; ============================================================================
;; Common Migration Steps
;; ============================================================================

(defn rename-field-migration
  "Create migration step to rename a field."
  [from-version to-version old-name new-name]
  (make-migration-step
    :from-version from-version
    :to-version to-version
    :transform-fn (fn [event]
                    (if (contains? event old-name)
                      (-> event
                          (assoc new-name (get event old-name))
                          (dissoc old-name))
                      event))
    :reversible true
    :reverse-fn (fn [event]
                  (if (contains? event new-name)
                    (-> event
                        (assoc old-name (get event new-name))
                        (dissoc new-name))
                    event))))

(defn add-field-migration
  "Create migration step to add a new field with default value."
  [from-version to-version field-name default-value]
  (make-migration-step
    :from-version from-version
    :to-version to-version
    :transform-fn (fn [event]
                    (if (contains? event field-name)
                      event
                      (assoc event field-name default-value)))
    :reversible true
    :reverse-fn (fn [event]
                  (dissoc event field-name))))

(defn remove-field-migration
  "Create migration step to remove a field."
  [from-version to-version field-name]
  (make-migration-step
    :from-version from-version
    :to-version to-version
    :transform-fn (fn [event]
                    (dissoc event field-name))
    :reversible false))

(defn transform-field-migration
  "Create migration step to transform a field value."
  [from-version to-version field-name transform-fn]
  (make-migration-step
    :from-version from-version
    :to-version to-version
    :transform-fn (fn [event]
                    (if (contains? event field-name)
                      (update event field-name transform-fn)
                      event))
    :reversible false))

;; ============================================================================
;; Accessors
;; ============================================================================

(defn get-replay-name
  "Get replay config name."
  [config]
  (:name config))

(defn get-validation-rules
  "Get validation rules from config."
  [config]
  (:rules config))

(defn get-migration-steps
  "Get migration steps from config."
  [config]
  (:steps config))

(defn get-schema-version
  "Get schema version."
  [schema]
  (:version schema))

(defn get-required-fields
  "Get required fields from schema."
  [schema]
  (:required schema))
