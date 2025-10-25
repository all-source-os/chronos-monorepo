(ns allsource.infrastructure.adapters.query-compiler
  "Compiles domain Query entities to Rust core API format."
  (:require [allsource.domain.entities.query :as q]
            [clojure.string :as str]))

(defn- compile-predicate
  "Compile a Predicate to Rust API filter format."
  [predicate]
  (let [operator (:operator predicate)
        field (:field predicate)
        value (:value predicate)]
    (case operator
      :eq {:field (name field) :op "eq" :value value}
      :ne {:field (name field) :op "ne" :value value}
      :gt {:field (name field) :op "gt" :value value}
      :gte {:field (name field) :op "gte" :value value}
      :lt {:field (name field) :op "lt" :value value}
      :lte {:field (name field) :op "lte" :value value}
      :contains {:field (name field) :op "contains" :value value}
      :in {:field (name field) :op "in" :value value}
      :not-in {:field (name field) :op "not_in" :value value}
      :between {:field (name field) :op "between" :value value}

      ;; Logical operators
      :and {:op "and" :predicates (mapv compile-predicate value)}
      :or {:op "or" :predicates (mapv compile-predicate value)}
      :not {:op "not" :predicate (compile-predicate value)}

      ;; Unknown operator
      (throw (ex-info "Unknown operator" {:operator operator})))))

(defn- compile-field
  "Compile a field selector to string format."
  [field]
  (cond
    (= :* field) "*"
    (keyword? field) (name field)
    (vector? field) (str/join "." (map name field))
    :else (str field)))

(defn- compile-select
  "Compile SELECT clause."
  [select-clause]
  (if (sequential? select-clause)
    (mapv compile-field select-clause)
    [(compile-field select-clause)]))

(defn- compile-order-by
  "Compile ORDER BY clause."
  [order-by-clause]
  (when order-by-clause
    (mapv (fn [sort-order]
            {:field (name (:field sort-order))
             :direction (name (:direction sort-order))})
          order-by-clause)))

(defn compile-to-rust-api
  "Compile a domain Query entity to Rust core API format.

   Returns a map suitable for JSON encoding and sending to Rust core:
   {
     :entity_id  (optional)
     :event_type (optional)
     :since      (optional timestamp)
     :until      (optional timestamp)
     :limit      (optional)
     :offset     (optional)
   }"
  [query]
  (let [base-query {}
        ;; Extract simple filters from WHERE clause
        where (:where query)]

    (cond-> base-query
      ;; Add entity_id filter if present
      (and where (= :eq (:operator where)) (= :entity-id (:field where)))
      (assoc :entity_id (:value where))

      ;; Add event_type filter if present
      (and where (= :eq (:operator where)) (= :event-type (:field where)))
      (assoc :event_type (:value where))

      ;; Add timestamp filters
      (and where (= :gt (:operator where)) (= :timestamp (:field where)))
      (assoc :since (:value where))

      (and where (= :lt (:operator where)) (= :timestamp (:field where)))
      (assoc :until (:value where))

      ;; Add limit
      (:limit query)
      (assoc :limit (:limit query))

      ;; Add offset
      (:offset query)
      (assoc :offset (:offset query))

      ;; For complex WHERE clauses, we'll need to fetch all and filter in Clojure
      ;; This is a simplification - can be optimized later
      (and where (#{:and :or :not} (:operator where)))
      (assoc :_complex_filter (compile-predicate where)))))

(defn needs-post-filter?
  "Check if query requires post-filtering in Clojure."
  [query]
  (let [where (:where query)]
    (and where
         (#{:and :or :not :contains :in :not-in :between} (:operator where)))))

(defn post-filter-events
  "Apply complex filters to events after fetching from Rust.
   This handles filters that can't be pushed down to Rust API."
  [events predicate]
  (letfn [(matches-predicate? [event pred]
            (let [operator (:operator pred)
                  field (:field pred)
                  value (:value pred)
                  event-value (get-in event (if (vector? field) field [field]))]
              (case operator
                :eq (= event-value value)
                :ne (not= event-value value)
                :gt (> event-value value)
                :gte (>= event-value value)
                :lt (< event-value value)
                :lte (<= event-value value)
                :contains (and (string? event-value)
                             (str/includes? event-value value))
                :in (contains? (set value) event-value)
                :not-in (not (contains? (set value) event-value))
                :between (let [[lower upper] value]
                          (and (>= event-value lower)
                               (<= event-value upper)))
                :and (every? #(matches-predicate? event %) value)
                :or (some #(matches-predicate? event %) value)
                :not (not (matches-predicate? event value))
                true)))]
    (filter #(matches-predicate? % predicate) events)))

(comment
  ;; Example compilation
  (compile-to-rust-api
    (q/make-query
      :select [:entity-id :event-type]
      :where (q/make-predicate :eq :event-type "user.created")
      :limit 100))
  ;; => {:event_type "user.created" :limit 100}

  (compile-to-rust-api
    (q/make-query
      :where (q/combine-predicates :and
                                   (q/make-predicate :eq :entity-id "user-123")
                                   (q/make-predicate :gt :timestamp #inst "2025-10-01"))))
  ;; => {:entity_id "user-123"
  ;;     :since #inst "2025-10-01"
  ;;     :_complex_filter {...}}
  )
