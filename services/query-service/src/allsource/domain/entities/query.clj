(ns allsource.domain.entities.query
  "Domain entities for query representation.
   Pure data structures with no external dependencies.")

;; Query Entity - represents a query against the event store
(defrecord Query [select from where order-by limit offset])

;; Predicate Entity - represents a condition in a WHERE clause
(defrecord Predicate [operator field value])

;; Aggregation Entity - represents an aggregation function
(defrecord Aggregation [function field alias])

;; Sort Order Entity
(defrecord SortOrder [field direction])

;; Query operators (domain constants)
(def operators
  #{:eq :ne :gt :gte :lt :lte
    :contains :in :not-in
    :and :or :not
    :between})

;; Aggregation functions (domain constants)
(def aggregation-functions
  #{:count :sum :avg :min :max
    :count-distinct :percentile
    :first :last})

;; Sort directions (domain constants)
(def sort-directions
  #{:asc :desc})

;; Pure domain functions for query validation

(defn valid-operator?
  "Check if operator is valid."
  [op]
  (contains? operators op))

(defn valid-aggregation-function?
  "Check if aggregation function is valid."
  [fn-name]
  (contains? aggregation-functions fn-name))

(defn valid-sort-direction?
  "Check if sort direction is valid."
  [dir]
  (contains? sort-directions dir))

(defn valid-predicate?
  "Validate a predicate entity."
  [predicate]
  (and (instance? Predicate predicate)
       (valid-operator? (:operator predicate))
       (some? (:field predicate))))

(defn valid-query?
  "Validate a query entity."
  [query]
  (and (instance? Query query)
       (or (nil? (:from query))
           (keyword? (:from query)))
       (or (nil? (:where query))
           (valid-predicate? (:where query)))))

;; Query builder helpers (pure functions)

(defn make-query
  "Create a new query entity."
  [& {:keys [select from where order-by limit offset]
      :or {select [:*]
           from :events
           where nil
           order-by nil
           limit nil
           offset nil}}]
  (map->Query {:select select
               :from from
               :where where
               :order-by order-by
               :limit limit
               :offset offset}))

(defn make-predicate
  "Create a new predicate entity."
  [operator field value]
  (map->Predicate {:operator operator
                   :field field
                   :value value}))

(defn make-aggregation
  "Create a new aggregation entity."
  [function field alias]
  (map->Aggregation {:function function
                     :field field
                     :alias alias}))

(defn make-sort-order
  "Create a new sort order entity."
  [field direction]
  (map->SortOrder {:field field
                   :direction direction}))

;; Query composition (pure functions)

(defn add-predicate
  "Add a WHERE predicate to a query."
  [query predicate]
  (if (valid-predicate? predicate)
    (assoc query :where predicate)
    (throw (ex-info "Invalid predicate" {:predicate predicate}))))

(defn add-order-by
  "Add an ORDER BY clause to a query."
  [query sort-orders]
  (assoc query :order-by sort-orders))

(defn add-limit
  "Add a LIMIT clause to a query."
  [query n]
  (assoc query :limit n))

(defn add-offset
  "Add an OFFSET clause to a query."
  [query n]
  (assoc query :offset n))

(defn combine-predicates
  "Combine multiple predicates with AND or OR."
  [operator & predicates]
  (when-not (#{:and :or} operator)
    (throw (ex-info "Operator must be :and or :or" {:operator operator})))
  (make-predicate operator nil predicates))
