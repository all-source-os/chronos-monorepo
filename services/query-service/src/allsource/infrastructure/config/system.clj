(ns allsource.infrastructure.config.system
  "System component for dependency injection using Component library.
   Wires together all the pieces of the application."
  (:require [com.stuartsierra.component :as component]
            [allsource.infrastructure.adapters.rust-core-client :as client]
            [clojure.tools.logging :as log]))

;; ============================================================================
;; Configuration Component
;; ============================================================================

(defrecord Config [env]
  component/Lifecycle

  (start [this]
    (log/info "Loading configuration" {:env env})
    (let [config (merge
                   ;; Defaults
                   {:rust-core-url "http://localhost:8080"
                    :auth-token (System/getenv "ALLSOURCE_AUTH_TOKEN")
                    :timeout-ms 5000
                    :max-connections 100
                    :log-level :info}
                   ;; Environment overrides
                   (when (= env :production)
                     {:rust-core-url (System/getenv "RUST_CORE_URL")
                      :max-connections 200
                      :timeout-ms 10000}))]
      (log/info "Configuration loaded" config)
      (assoc this :config config)))

  (stop [this]
    (log/info "Configuration stopped")
    (dissoc this :config)))

(defn new-config [env]
  (map->Config {:env env}))

;; ============================================================================
;; Query Client Component
;; ============================================================================

(defrecord QueryClient [config client]
  component/Lifecycle

  (start [this]
    (log/info "Starting QueryClient")
    (let [cfg (:config config)
          query-client (client/create-client
                         {:base-url (:rust-core-url cfg)
                          :auth-token (:auth-token cfg)
                          :timeout-ms (:timeout-ms cfg)
                          :max-connections (:max-connections cfg)})]
      (log/info "QueryClient started")
      (assoc this :client query-client)))

  (stop [this]
    (log/info "Stopping QueryClient")
    (when-let [c (:client this)]
      (client/close-client c))
    (dissoc this :client)))

(defn new-query-client []
  (map->QueryClient {}))

;; ============================================================================
;; System Assembly
;; ============================================================================

(defn new-system
  "Create a new system with all components wired together.

   Usage:
   (def system (new-system :development))
   (alter-var-root #'system component/start)
   (alter-var-root #'system component/stop)"
  [env]
  (component/system-map
    :config (new-config env)
    :query-client (component/using
                    (new-query-client)
                    [:config])))

;; ============================================================================
;; System Management Functions
;; ============================================================================

(def ^:dynamic *system* nil)

(defn start-system!
  "Start the system and bind it to *system*."
  ([env]
   (log/info "Starting AllSource Query Service" {:env env})
   (let [system (component/start (new-system env))]
     (alter-var-root #'*system* (constantly system))
     (log/info "System started successfully")
     system)))

(defn stop-system!
  "Stop the current system."
  []
  (when *system*
    (log/info "Stopping AllSource Query Service")
    (component/stop *system*)
    (alter-var-root #'*system* (constantly nil))
    (log/info "System stopped")))

(defn restart-system!
  "Restart the system (stop then start)."
  [env]
  (stop-system!)
  (start-system! env))

(defn get-query-client
  "Get the query client from the running system."
  []
  (if *system*
    (get-in *system* [:query-client :client])
    (throw (ex-info "System not started. Call (start-system! :development) first." {}))))

(comment
  ;; Start the system
  (start-system! :development)

  ;; Get the query client
  (def client (get-query-client))

  ;; Use the client
  (require '[allsource.application.dsl :as dsl])
  (require '[allsource.domain.protocols.query-executor :as qe])

  (qe/execute-query
    client
    (dsl/query {:from :events
                :where [:= :event-type "user.created"]
                :limit 10}))

  ;; Stop the system
  (stop-system!)
  )
