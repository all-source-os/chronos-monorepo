(ns allsource.infrastructure.adapters.redis-state-store
  "Redis-based state store implementation.
   Fast, in-memory cache for projection state with optional TTL."
  (:require [allsource.domain.protocols.projection-executor :as pe]
            [cheshire.core :as json]
            [taoensso.carmine :as car :refer [wcar]])
  (:import [java.time Instant]))

;; ============================================================================
;; Redis Connection Configuration
;; ============================================================================

(defn create-redis-conn-spec
  "Create a Redis connection specification.

   Options:
   - :host - Redis host (default localhost)
   - :port - Redis port (default 6379)
   - :password - Redis password (optional)
   - :db - Redis database number (default 0)
   - :timeout-ms - Connection timeout (default 4000)
   - :pool-size - Connection pool size (default 10)"
  [{:keys [host port password db timeout-ms pool-size]
    :or {host "localhost"
         port 6379
         db 0
         timeout-ms 4000
         pool-size 10}}]
  {:pool {:max-active pool-size
          :max-idle (quot pool-size 2)
          :min-idle 1
          :max-wait-ms timeout-ms}
   :spec {:host host
          :port port
          :password password
          :db db
          :timeout-ms timeout-ms}})

;; ============================================================================
;; Key Generation
;; ============================================================================

(defn- state-key
  "Generate Redis key for projection state."
  [projection-name entity-id]
  (str "projection:state:" (name projection-name) ":" entity-id))

(defn- snapshot-key
  "Generate Redis key for projection snapshot."
  [projection-name entity-id]
  (str "projection:snapshot:" (name projection-name) ":" entity-id))

(defn- projection-entities-key
  "Generate Redis set key for tracking all entities in a projection."
  [projection-name]
  (str "projection:entities:" (name projection-name)))

;; ============================================================================
;; Redis State Store Implementation
;; ============================================================================

(defrecord RedisStateStore [conn-spec ttl-seconds]
  pe/StateStore

  (save-state [this projection-name entity-id state version]
    (try
      (let [key (state-key projection-name entity-id)
            entities-key (projection-entities-key projection-name)
            state-data {:state state
                        :version version
                        :updated-at (.toString (Instant/now))}
            json-data (json/generate-string state-data)]
        (wcar conn-spec
              ;; Save state
              (car/set key json-data)
              ;; Set TTL if configured
              (when ttl-seconds
                (car/expire key ttl-seconds))
              ;; Track entity in projection set
              (car/sadd entities-key (str entity-id)))
        {:success true})
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (load-state [this projection-name entity-id]
    (try
      (let [key (state-key projection-name entity-id)
            json-data (wcar conn-spec (car/get key))]
        (when json-data
          (let [data (json/parse-string json-data true)]
            (:state data))))
      (catch Exception e
        nil)))

  (delete-state [this projection-name entity-id]
    (try
      (let [key (state-key projection-name entity-id)
            entities-key (projection-entities-key projection-name)]
        (wcar conn-spec
              (car/del key)
              (car/srem entities-key (str entity-id)))
        {:success true})
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (save-snapshot [this snapshot]
    (try
      (let [projection-name (:projection-name snapshot)
            entity-id (:entity-id snapshot)
            key (snapshot-key projection-name entity-id)
            snapshot-id (str (java.util.UUID/randomUUID))
            snapshot-data (assoc snapshot
                                 :snapshot-id snapshot-id
                                 :timestamp (.toString (:timestamp snapshot)))
            json-data (json/generate-string snapshot-data)]
        (wcar conn-spec
              (car/set key json-data)
              ;; Snapshots should persist longer
              (when ttl-seconds
                (car/expire key (* ttl-seconds 10))))
        {:success true :snapshot-id snapshot-id})
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (load-snapshot [this projection-name entity-id]
    (try
      (let [key (snapshot-key projection-name entity-id)
            json-data (wcar conn-spec (car/get key))]
        (when json-data
          (let [data (json/parse-string json-data true)]
            (update data :timestamp #(Instant/parse %)))))
      (catch Exception e
        nil)))

  (list-all-states [this projection-name]
    (try
      (let [entities-key (projection-entities-key projection-name)
            entity-ids (wcar conn-spec (car/smembers entities-key))]
        (for [entity-id entity-ids
              :let [state (pe/load-state this projection-name entity-id)]
              :when state]
          [entity-id state]))
      (catch Exception e
        [])))

  java.io.Closeable
  (close [this]
    ;; Carmine handles connection pooling automatically
    nil))

;; ============================================================================
;; Constructor and Utilities
;; ============================================================================

(defn create-redis-state-store
  "Create a new Redis state store.

   Options (all optional):
   - :host - Redis host (default localhost)
   - :port - Redis port (default 6379)
   - :password - Redis password (optional)
   - :db - Redis database number (default 0)
   - :ttl-seconds - TTL for state keys (default nil = no expiration)
   - :pool-size - Connection pool size (default 10)

   Environment variables:
   - REDIS_HOST
   - REDIS_PORT
   - REDIS_PASSWORD
   - REDIS_DB
   - REDIS_TTL_SECONDS"
  ([]
   (create-redis-state-store {}))
  ([options]
   (let [host (or (:host options)
                  (System/getenv "REDIS_HOST")
                  "localhost")
         port (or (:port options)
                  (when-let [p (System/getenv "REDIS_PORT")]
                    (Integer/parseInt p))
                  6379)
         password (or (:password options)
                      (System/getenv "REDIS_PASSWORD"))
         db (or (:db options)
                (when-let [d (System/getenv "REDIS_DB")]
                  (Integer/parseInt d))
                0)
         ttl-seconds (or (:ttl-seconds options)
                         (when-let [t (System/getenv "REDIS_TTL_SECONDS")]
                           (Integer/parseInt t)))
         pool-size (or (:pool-size options) 10)
         conn-spec (create-redis-conn-spec
                     {:host host
                      :port port
                      :password password
                      :db db
                      :pool-size pool-size})]
     (->RedisStateStore conn-spec ttl-seconds))))

;; ============================================================================
;; Redis-Specific Utilities
;; ============================================================================

(defn get-cache-statistics
  "Get cache statistics for a projection.

   Returns:
   - :entity-count - Number of entities in cache
   - :memory-usage - Approximate memory usage in bytes
   - :hit-rate - Cache hit rate (if available)"
  [store projection-name]
  (let [conn-spec (:conn-spec store)
        entities-key (projection-entities-key projection-name)]
    (try
      (let [entity-count (wcar conn-spec (car/scard entities-key))
            ;; Get approximate memory usage
            info (wcar conn-spec (car/info "memory"))
            memory-usage (when info
                           (some-> info
                                   (clojure.string/split-lines)
                                   (->> (filter #(clojure.string/starts-with? % "used_memory:")))
                                   first
                                   (clojure.string/split #":")
                                   second
                                   Long/parseLong))]
        {:entity-count entity-count
         :memory-usage memory-usage})
      (catch Exception e
        {:entity-count 0
         :memory-usage 0}))))

(defn flush-projection
  "Flush all state for a projection from Redis cache.

   Useful for cache invalidation or resets."
  [store projection-name]
  (let [conn-spec (:conn-spec store)
        entities-key (projection-entities-key projection-name)]
    (try
      (let [entity-ids (wcar conn-spec (car/smembers entities-key))]
        ;; Delete all state keys
        (doseq [entity-id entity-ids]
          (let [state-key (state-key projection-name entity-id)
                snapshot-key (snapshot-key projection-name entity-id)]
            (wcar conn-spec
                  (car/del state-key)
                  (car/del snapshot-key))))
        ;; Delete entities set
        (wcar conn-spec (car/del entities-key))
        {:success true :deleted-count (count entity-ids)})
      (catch Exception e
        {:success false :error (.getMessage e)}))))

(defn warm-cache
  "Warm the Redis cache with data from a cold store.

   Parameters:
   - store: RedisStateStore instance
   - projection-name: Projection to warm
   - fetch-fn: Function (projection-name) => [[entity-id state] ...]

   Returns: {:success true :loaded-count <n>}"
  [store projection-name fetch-fn]
  (try
    (let [states (fetch-fn projection-name)
          version 1] ; Default version for warmed cache
      (doseq [[entity-id state] states]
        (pe/save-state store projection-name entity-id state version))
      {:success true :loaded-count (count states)})
    (catch Exception e
      {:success false :error (.getMessage e)})))

(defn set-ttl
  "Update TTL for a specific entity's state.

   Useful for extending cache lifetime for hot data."
  [store projection-name entity-id ttl-seconds]
  (let [conn-spec (:conn-spec store)
        key (state-key projection-name entity-id)]
    (try
      (wcar conn-spec (car/expire key ttl-seconds))
      {:success true}
      (catch Exception e
        {:success false :error (.getMessage e)}))))

(defn get-ttl
  "Get remaining TTL for an entity's state.

   Returns: TTL in seconds, or nil if no TTL or key doesn't exist"
  [store projection-name entity-id]
  (let [conn-spec (:conn-spec store)
        key (state-key projection-name entity-id)]
    (try
      (let [ttl (wcar conn-spec (car/ttl key))]
        (when (and ttl (pos? ttl))
          ttl))
      (catch Exception e
        nil))))
