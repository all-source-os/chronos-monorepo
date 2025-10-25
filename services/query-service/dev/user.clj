(ns user
  "Development namespace with REPL helpers.
   This namespace is automatically loaded when starting a REPL."
  (:require [allsource.infrastructure.config.system :as sys]
            [allsource.application.dsl :as dsl]
            [allsource.domain.protocols.query-executor :as qe]
            [clojure.tools.logging :as log]
            [clojure.pprint :refer [pprint]]
            [fipp.edn :as fipp]))

;; ============================================================================
;; REPL Startup
;; ============================================================================

(println "\n╔══════════════════════════════════════════════════════════════════╗")
(println "║  AllSource Query Service - Interactive REPL                     ║")
(println "║                                                                  ║")
(println "║  Quick Start:                                                    ║")
(println "║    (start!)       - Start the system                            ║")
(println "║    (stop!)        - Stop the system                             ║")
(println "║    (restart!)     - Restart the system                          ║")
(println "║    (recent 10)    - Get 10 most recent events                   ║")
(println "║    (help)         - Show all available commands                 ║")
(println "╚══════════════════════════════════════════════════════════════════╝\n")

;; ============================================================================
;; System Management
;; ============================================================================

(defn start!
  "Start the AllSource Query Service system."
  []
  (sys/start-system! :development)
  (println "✓ System started")
  :started)

(defn stop!
  "Stop the AllSource Query Service system."
  []
  (sys/stop-system!)
  (println "✓ System stopped")
  :stopped)

(defn restart!
  "Restart the system."
  []
  (stop!)
  (start!)
  :restarted)

(defn status
  "Check system status."
  []
  (if sys/*system*
    (do
      (println "System Status: Running")
      (pprint (keys sys/*system*))
      :running)
    (do
      (println "System Status: Stopped")
      :stopped)))

;; ============================================================================
;; Query Helpers
;; ============================================================================

(defn execute!
  "Execute a query and return results.

   Example:
   (execute! (dsl/query {:from :events :limit 10}))"
  [query]
  (let [client (sys/get-query-client)]
    (qe/execute-query client query)))

(defn recent
  "Get n most recent events.

   Example:
   (recent 10)"
  [n]
  (execute! (-> (dsl/from-events)
                (dsl/order-by-timestamp :desc)
                (dsl/limit n))))

(defn by-type
  "Get events by event type.

   Example:
   (by-type \"user.created\" 50)"
  ([event-type] (by-type event-type 100))
  ([event-type limit]
   (execute! (dsl/query {:from :events
                         :where [:= :event-type event-type]
                         :order-by [[:timestamp :desc]]
                         :limit limit}))))

(defn by-entity
  "Get all events for an entity.

   Example:
   (by-entity \"user-123\")"
  ([entity-id] (by-entity entity-id 1000))
  ([entity-id limit]
   (execute! (dsl/query {:from :events
                         :where [:= :entity-id entity-id]
                         :order-by [[:timestamp :asc]]
                         :limit limit}))))

(defn since
  "Get events since a timestamp.

   Example:
   (since (dsl/days-ago 7) 100)"
  ([timestamp] (since timestamp 100))
  ([timestamp limit]
   (execute! (-> (dsl/from-events)
                 (dsl/since timestamp)
                 (dsl/order-by-timestamp :desc)
                 (dsl/limit limit)))))

(defn recent-by-type
  "Get recent events of a specific type.

   Example:
   (recent-by-type \"order.placed\" 20)"
  ([event-type n]
   (execute! (dsl/query {:from :events
                         :where [:= :event-type event-type]
                         :order-by [[:timestamp :desc]]
                         :limit n}))))

(defn count-by-type
  "Count events by type in the last n days.

   Example:
   (count-by-type 7)"
  [days]
  (let [events (execute! (-> (dsl/from-events)
                             (dsl/since (dsl/days-ago days))
                             (dsl/select [:event-type])))]
    (frequencies (map :event-type events))))

(defn today
  "Get today's events.

   Example:
   (today 100)"
  ([limit]
   (execute! (-> (dsl/from-events)
                 (dsl/since (dsl/days-ago 1))
                 (dsl/order-by-timestamp :desc)
                 (dsl/limit limit)))))

;; ============================================================================
;; Pretty Printing
;; ============================================================================

(defn pp
  "Pretty print with fipp (better formatting than pprint).

   Example:
   (pp (recent 5))"
  [data]
  (fipp/pprint data))

(defn show
  "Show first n events with pretty printing.

   Example:
   (show (recent 100) 10)"
  ([events] (show events 10))
  ([events n]
   (pp (take n events))))

;; ============================================================================
;; Query Building Examples
;; ============================================================================

(defn example-simple
  "Simple query example."
  []
  (dsl/query
    {:select [:entity-id :event-type :timestamp]
     :from :events
     :where [:= :event-type "user.created"]
     :limit 10}))

(defn example-complex
  "Complex query with AND/OR example."
  []
  (dsl/query
    {:from :events
     :where [:and
             [:= :event-type "order.placed"]
             [:> :timestamp (dsl/days-ago 7)]
             [:or
              [:> [:payload :amount] 1000]
              [:contains [:payload :tags] "premium"]]]
     :order-by [[:timestamp :desc]]
     :limit 50}))

(defn example-fluent
  "Fluent query building example."
  []
  (-> (dsl/from-events)
      (dsl/select [:entity-id :event-type :payload])
      (dsl/where [:= :event-type "order.placed"])
      (dsl/since (dsl/days-ago 30))
      (dsl/order-by-timestamp)
      (dsl/limit 100)))

;; ============================================================================
;; Help Function
;; ============================================================================

(defn help
  "Show all available REPL commands."
  []
  (println "\n=== System Management ===")
  (println "(start!)              - Start the system")
  (println "(stop!)               - Stop the system")
  (println "(restart!)            - Restart the system")
  (println "(status)              - Check system status")
  (println "\n=== Query Execution ===")
  (println "(execute! query)      - Execute a query")
  (println "(recent n)            - Get n most recent events")
  (println "(by-type \"type\" n)    - Get n events of type")
  (println "(by-entity \"id\" n)    - Get n events for entity")
  (println "(since timestamp n)   - Get n events since timestamp")
  (println "(today n)             - Get today's events")
  (println "\n=== Analysis ===")
  (println "(count-by-type days)  - Count events by type in last n days")
  (println "\n=== Pretty Printing ===")
  (println "(pp data)             - Pretty print data")
  (println "(show events n)       - Show first n events")
  (println "\n=== Examples ===")
  (println "(example-simple)      - See simple query example")
  (println "(example-complex)     - See complex query example")
  (println "(example-fluent)      - See fluent query building")
  (println "\n=== Time Helpers ===")
  (println "(dsl/days-ago n)      - Timestamp n days ago")
  (println "(dsl/hours-ago n)     - Timestamp n hours ago")
  (println "(dsl/now)             - Current timestamp")
  (println))

;; ============================================================================
;; REPL Configuration
;; ============================================================================

;; Set pretty printing defaults
(set! *print-length* 50)
(set! *print-level* 5)

(comment
  ;; Start system and try some queries
  (start!)

  ;; Get recent events
  (pp (recent 10))

  ;; Get events by type
  (pp (by-type "user.created" 5))

  ;; Build and execute custom query
  (pp (execute!
        (-> (dsl/from-events)
            (dsl/where [:= :event-type "order.placed"])
            (dsl/since (dsl/days-ago 7))
            (dsl/limit 20))))

  ;; Count events by type
  (count-by-type 30)

  ;; Stop system when done
  (stop!)
  )
