(ns allsource.infrastructure.adapters.rust-core-client
  "HTTP client adapter for communicating with Rust core event store.
   Implements the QueryExecutor protocol."
  (:require [clj-http.client :as http]
            [cheshire.core :as json]
            [clojure.tools.logging :as log]
            [allsource.domain.protocols.query-executor :as qe]
            [allsource.infrastructure.adapters.query-compiler :as compiler]))

(defrecord RustCoreClient [base-url auth-token timeout-ms connection-pool]
  qe/QueryExecutor

  (execute-query [this query]
    (log/info "Executing query against Rust core" {:query query})
    (let [compiled-query (compiler/compile-to-rust-api query)
          response (http/post (str base-url "/api/v1/events/query")
                              {:headers {"Authorization" (str "Bearer " auth-token)
                                        "Content-Type" "application/json"}
                               :body (json/generate-string compiled-query)
                               :timeout timeout-ms
                               :as :json
                               :throw-exceptions false
                               :connection-manager connection-pool})]
      (cond
        (= 200 (:status response))
        (do
          (log/debug "Query successful" {:count (count (get-in response [:body :events]))})
          (get-in response [:body :events]))

        (= 401 (:status response))
        (throw (ex-info "Unauthorized: Invalid or expired auth token"
                       {:status 401 :response (:body response)}))

        (= 403 (:status response))
        (throw (ex-info "Forbidden: Insufficient permissions"
                       {:status 403 :response (:body response)}))

        (= 400 (:status response))
        (throw (ex-info "Bad request: Invalid query"
                       {:status 400 :response (:body response) :query compiled-query}))

        :else
        (throw (ex-info "Query execution failed"
                       {:status (:status response)
                        :response (:body response)})))))

  (execute-query-async [this query callback]
    (future
      (try
        (let [results (qe/execute-query this query)]
          (callback {:success true :results results}))
        (catch Exception e
          (log/error e "Async query execution failed")
          (callback {:success false :error e})))))

  (stream-query-results [this query]
    ;; For now, return lazy sequence. Can optimize later with chunked transfers
    (lazy-seq (qe/execute-query this query)))

  (compile-query [this query]
    (compiler/compile-to-rust-api query)))

;; ============================================================================
;; Client Factory
;; ============================================================================

(defn create-client
  "Create a new RustCoreClient with connection pooling.

   Options:
   - :base-url - Rust core API endpoint (required)
   - :auth-token - JWT token for authentication (required)
   - :timeout-ms - Request timeout in milliseconds (default: 5000)
   - :max-connections - Max connection pool size (default: 100)
   - :connection-timeout-ms - Connection timeout (default: 5000)"
  [{:keys [base-url auth-token timeout-ms max-connections connection-timeout-ms]
    :or {timeout-ms 5000
         max-connections 100
         connection-timeout-ms 5000}}]
  (when-not base-url
    (throw (ex-info "base-url is required" {})))
  (when-not auth-token
    (throw (ex-info "auth-token is required" {})))

  (let [connection-pool (http/make-reusable-conn-manager
                          {:timeout connection-timeout-ms
                           :threads max-connections})]
    (log/info "Created RustCoreClient"
              {:base-url base-url
               :timeout-ms timeout-ms
               :max-connections max-connections})
    (map->RustCoreClient {:base-url base-url
                          :auth-token auth-token
                          :timeout-ms timeout-ms
                          :connection-pool connection-pool})))

(defn close-client
  "Close the HTTP client and release resources."
  [client]
  (when-let [pool (:connection-pool client)]
    (log/info "Closing RustCoreClient connection pool")
    (.shutdown pool)))

;; ============================================================================
;; Health Check
;; ============================================================================

(defn health-check
  "Check if the Rust core is healthy and reachable."
  [client]
  (try
    (let [response (http/get (str (:base-url client) "/health")
                             {:timeout 2000
                              :throw-exceptions false
                              :connection-manager (:connection-pool client)})]
      {:healthy? (= 200 (:status response))
       :status (:status response)
       :response-time-ms (get-in response [:headers "x-response-time"])})
    (catch Exception e
      (log/error e "Health check failed")
      {:healthy? false
       :error (.getMessage e)})))
