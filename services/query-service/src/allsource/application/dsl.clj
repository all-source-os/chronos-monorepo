(ns allsource.application.dsl
  "Query DSL for building queries in a fluent, declarative style.
   This is the main user-facing API."
  (:require [allsource.domain.entities.query :as q]
            [java-time.api :as jt]))

;; ============================================================================
;; Query Builder DSL
;; ============================================================================

(defn query
  "Create a new query from a map specification.

   Example:
   (query {:select [:entity-id :event-type]
           :from :events
           :where [:= :event-type \"user.created\"]
           :limit 100})"
  [{:keys [select from where order-by limit offset]
    :or {select [:*]
         from :events}}]
  (let [base-query (q/make-query :select select :from from)]
    (cond-> base-query
      where (q/add-predicate (parse-where where))
      order-by (q/add-order-by (parse-order-by order-by))
      limit (q/add-limit limit)
      offset (q/add-offset offset))))

(defn from-events
  "Start building a query from events table."
  []
  (q/make-query :from :events))

(defn from-projections
  "Start building a query from a projection.

   Example:
   (from-projections :user-statistics)"
  [projection-name]
  (q/make-query :from (keyword "projections" (name projection-name))))

;; ============================================================================
;; WHERE Clause Builders
;; ============================================================================

(defn parse-where
  "Parse a where clause from vector notation to Predicate.

   Supports:
   [:= :field value]
   [:> :field value]
   [:and [:= :a 1] [:= :b 2]]
   [:or [:= :a 1] [:= :b 2]]"
  [where-clause]
  (cond
    (vector? where-clause)
    (let [[op & args] where-clause]
      (case op
        (:and :or) (apply q/combine-predicates op (map parse-where args))
        (:eq :ne :gt :gte :lt :lte :contains :in :not-in :between)
        (apply q/make-predicate op args)
        (throw (ex-info "Unknown operator" {:operator op}))))

    :else
    (throw (ex-info "Where clause must be a vector" {:clause where-clause}))))

(defn where
  "Add a WHERE clause to a query.

   Example:
   (-> (from-events)
       (where [:= :event-type \"user.created\"]))"
  [query-or-clause predicate-clause]
  (if (instance? allsource.domain.entities.query.Query query-or-clause)
    (q/add-predicate query-or-clause (parse-where predicate-clause))
    ;; If just a predicate, return it
    (parse-where query-or-clause)))

;; ============================================================================
;; Comparison Operators
;; ============================================================================

(defn eq
  "Equal comparison."
  [field value]
  [:= field value])

(defn ne
  "Not equal comparison."
  [field value]
  [:ne field value])

(defn gt
  "Greater than comparison."
  [field value]
  [:> field value])

(defn gte
  "Greater than or equal comparison."
  [field value]
  [:>= field value])

(defn lt
  "Less than comparison."
  [field value]
  [:< field value])

(defn lte
  "Less than or equal comparison."
  [field value]
  [:<= field value])

(defn contains?
  "String contains comparison."
  [field substring]
  [:contains field substring])

(defn in?
  "IN operator (value in list)."
  [field values]
  [:in field values])

(defn between
  "BETWEEN operator."
  [field lower upper]
  [:between field lower upper])

;; ============================================================================
;; Logical Operators
;; ============================================================================

(defn and
  "Combine predicates with AND."
  [& predicates]
  (vec (cons :and predicates)))

(defn or
  "Combine predicates with OR."
  [& predicates]
  (vec (cons :or predicates)))

(defn not
  "Negate a predicate."
  [predicate]
  [:not predicate])

;; ============================================================================
;; SELECT Clause
;; ============================================================================

(defn select
  "Specify which fields to select.

   Example:
   (-> (from-events)
       (select [:entity-id :event-type :timestamp]))"
  [query fields]
  (assoc query :select fields))

(defn select-all
  "Select all fields."
  [query]
  (assoc query :select [:*]))

;; ============================================================================
;; ORDER BY Clause
;; ============================================================================

(defn parse-order-by
  "Parse order-by clause.

   Examples:
   [[:timestamp :desc]]
   [[:timestamp :desc] [:entity-id :asc]]"
  [order-by-clause]
  (mapv (fn [[field dir]]
          (q/make-sort-order field (or dir :asc)))
        order-by-clause))

(defn order-by
  "Add ORDER BY clause.

   Example:
   (-> (from-events)
       (order-by [[:timestamp :desc]]))"
  [query order-spec]
  (q/add-order-by query (parse-order-by order-spec)))

(defn order-by-timestamp
  "Order by timestamp (descending by default)."
  ([query] (order-by-timestamp query :desc))
  ([query direction]
   (order-by query [[:timestamp direction]])))

;; ============================================================================
;; LIMIT and OFFSET
;; ============================================================================

(defn limit
  "Add LIMIT clause.

   Example:
   (-> (from-events) (limit 100))"
  [query n]
  (q/add-limit query n))

(defn offset
  "Add OFFSET clause for pagination.

   Example:
   (-> (from-events) (limit 100) (offset 200))"
  [query n]
  (q/add-offset query n))

(defn take-n
  "Alias for limit."
  [query n]
  (limit query n))

;; ============================================================================
;; Time-based Helpers
;; ============================================================================

(defn now
  "Current timestamp."
  []
  (jt/instant))

(defn days-ago
  "Timestamp n days ago."
  [n]
  (jt/minus (jt/instant) (jt/days n)))

(defn hours-ago
  "Timestamp n hours ago."
  [n]
  (jt/minus (jt/instant) (jt/hours n)))

(defn minutes-ago
  "Timestamp n minutes ago."
  [n]
  (jt/minus (jt/instant) (jt/minutes n)))

(defn since
  "Events since timestamp.

   Example:
   (-> (from-events)
       (where [:> :timestamp (days-ago 7)]))"
  [query timestamp]
  (where query [:> :timestamp timestamp]))

(defn until
  "Events until timestamp."
  [query timestamp]
  (where query [:< :timestamp timestamp]))

(defn time-range
  "Events in time range."
  [query start end]
  (where query [:and
                [:>= :timestamp start]
                [:<= :timestamp end]]))

;; ============================================================================
;; Aggregation Helpers
;; ============================================================================

(defn count-events
  "Count aggregation."
  ([] (q/make-aggregation :count :* :count))
  ([field] (q/make-aggregation :count field :count)))

(defn sum-field
  "Sum aggregation."
  [field]
  (q/make-aggregation :sum field (keyword (str (name field) "-sum"))))

(defn avg-field
  "Average aggregation."
  [field]
  (q/make-aggregation :avg field (keyword (str (name field) "-avg"))))

(defn min-field
  "Minimum aggregation."
  [field]
  (q/make-aggregation :min field (keyword (str (name field) "-min"))))

(defn max-field
  "Maximum aggregation."
  [field]
  (q/make-aggregation :max field (keyword (str (name field) "-max"))))

(defn count-distinct
  "Count distinct values."
  [field]
  (q/make-aggregation :count-distinct field :distinct-count))

;; ============================================================================
;; Examples in Comments
;; ============================================================================

(comment
  ;; Simple query
  (query
    {:select [:entity-id :event-type :timestamp]
     :from :events
     :where [:= :event-type "user.created"]
     :limit 100})

  ;; Fluent query building
  (-> (from-events)
      (select [:entity-id :event-type])
      (where [:= :event-type "user.created"])
      (order-by-timestamp :desc)
      (limit 100))

  ;; Complex query with AND/OR
  (query
    {:from :events
     :where [:and
             [:= :event-type "order.placed"]
             [:> :timestamp (days-ago 7)]
             [:or
              [:> [:payload :amount] 1000]
              [:contains [:payload :tags] "premium"]]]
     :order-by [[:timestamp :desc]]
     :limit 50})

  ;; Time-based query
  (-> (from-events)
      (where [:= :event-type "order.placed"])
      (since (days-ago 30))
      (order-by-timestamp)
      (limit 1000))

  )
