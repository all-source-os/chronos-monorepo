(ns allsource.infrastructure.adapters.postgres-state-store
  "PostgreSQL-based state store implementation.
   Production-ready persistence for projection state with ACID guarantees."
  (:require [allsource.domain.protocols.projection-executor :as pe]
            [clojure.java.jdbc :as jdbc]
            [cheshire.core :as json])
  (:import [java.time Instant]
           [com.zaxxer.hikari HikariConfig HikariDataSource]))

;; ============================================================================
;; Connection Pool Management
;; ============================================================================

(defn create-connection-pool
  "Create a HikariCP connection pool for PostgreSQL.

   Options:
   - :connection-string - JDBC connection string
   - :username - Database username
   - :password - Database password
   - :pool-size - Maximum pool size (default 10)
   - :connection-timeout - Connection timeout in ms (default 30000)
   - :idle-timeout - Idle timeout in ms (default 600000)
   - :max-lifetime - Maximum connection lifetime in ms (default 1800000)"
  [{:keys [connection-string username password pool-size connection-timeout
           idle-timeout max-lifetime]
    :or {pool-size 10
         connection-timeout 30000
         idle-timeout 600000
         max-lifetime 1800000}}]
  (let [config (doto (HikariConfig.)
                 (.setJdbcUrl connection-string)
                 (.setUsername username)
                 (.setPassword password)
                 (.setMaximumPoolSize pool-size)
                 (.setConnectionTimeout connection-timeout)
                 (.setIdleTimeout idle-timeout)
                 (.setMaxLifetime max-lifetime)
                 (.setAutoCommit true)
                 (.setConnectionTestQuery "SELECT 1"))]
    (HikariDataSource. config)))

(defn close-connection-pool
  "Close the connection pool."
  [^HikariDataSource datasource]
  (when datasource
    (.close datasource)))

;; ============================================================================
;; Schema Initialization
;; ============================================================================

(def ^:private projection-state-table-ddl
  "CREATE TABLE IF NOT EXISTS projection_states (
     projection_name TEXT NOT NULL,
     entity_id TEXT NOT NULL,
     state JSONB NOT NULL,
     version INTEGER NOT NULL,
     updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
     PRIMARY KEY (projection_name, entity_id)
   );

   CREATE INDEX IF NOT EXISTS idx_projection_states_projection
     ON projection_states(projection_name);

   CREATE INDEX IF NOT EXISTS idx_projection_states_updated
     ON projection_states(projection_name, updated_at DESC);")

(def ^:private projection-snapshot-table-ddl
  "CREATE TABLE IF NOT EXISTS projection_snapshots (
     snapshot_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
     projection_name TEXT NOT NULL,
     entity_id TEXT NOT NULL,
     state JSONB NOT NULL,
     version INTEGER NOT NULL,
     timestamp TIMESTAMP NOT NULL,
     created_at TIMESTAMP NOT NULL DEFAULT NOW(),
     UNIQUE(projection_name, entity_id)
   );

   CREATE INDEX IF NOT EXISTS idx_projection_snapshots_projection
     ON projection_snapshots(projection_name);")

(defn initialize-schema
  "Initialize PostgreSQL schema for projection state storage."
  [db-spec]
  (jdbc/execute! db-spec [projection-state-table-ddl])
  (jdbc/execute! db-spec [projection-snapshot-table-ddl]))

;; ============================================================================
;; PostgreSQL State Store Implementation
;; ============================================================================

(defrecord PostgresStateStore [datasource db-spec]
  pe/StateStore

  (save-state [this projection-name entity-id state version]
    (try
      (jdbc/execute! db-spec
                     ["INSERT INTO projection_states
                       (projection_name, entity_id, state, version, updated_at)
                       VALUES (?, ?, ?::jsonb, ?, NOW())
                       ON CONFLICT (projection_name, entity_id)
                       DO UPDATE SET
                         state = EXCLUDED.state,
                         version = EXCLUDED.version,
                         updated_at = NOW()"
                      (name projection-name)
                      (str entity-id)
                      (json/generate-string state)
                      version])
      {:success true}
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (load-state [this projection-name entity-id]
    (try
      (let [result (jdbc/query db-spec
                               ["SELECT state FROM projection_states
                                 WHERE projection_name = ? AND entity_id = ?"
                                (name projection-name)
                                (str entity-id)])]
        (when-let [row (first result)]
          (json/parse-string (:state row) true)))
      (catch Exception e
        nil)))

  (delete-state [this projection-name entity-id]
    (try
      (jdbc/execute! db-spec
                     ["DELETE FROM projection_states
                       WHERE projection_name = ? AND entity_id = ?"
                      (name projection-name)
                      (str entity-id)])
      {:success true}
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (save-snapshot [this snapshot]
    (try
      (let [result (jdbc/query db-spec
                               ["INSERT INTO projection_snapshots
                                 (projection_name, entity_id, state, version, timestamp)
                                 VALUES (?, ?, ?::jsonb, ?, ?::timestamp)
                                 ON CONFLICT (projection_name, entity_id)
                                 DO UPDATE SET
                                   state = EXCLUDED.state,
                                   version = EXCLUDED.version,
                                   timestamp = EXCLUDED.timestamp,
                                   created_at = NOW()
                                 RETURNING snapshot_id"
                                (name (:projection-name snapshot))
                                (str (:entity-id snapshot))
                                (json/generate-string (:state snapshot))
                                (:version snapshot)
                                (.toString (:timestamp snapshot))])]
        {:success true
         :snapshot-id (str (:snapshot_id (first result)))})
      (catch Exception e
        {:success false :error (.getMessage e)})))

  (load-snapshot [this projection-name entity-id]
    (try
      (let [result (jdbc/query db-spec
                               ["SELECT * FROM projection_snapshots
                                 WHERE projection_name = ? AND entity_id = ?
                                 ORDER BY created_at DESC
                                 LIMIT 1"
                                (name projection-name)
                                (str entity-id)])]
        (when-let [row (first result)]
          {:projection-name (keyword (:projection_name row))
           :entity-id (:entity_id row)
           :state (json/parse-string (:state row) true)
           :version (:version row)
           :timestamp (Instant/parse (str (:timestamp row)))
           :snapshot-id (str (:snapshot_id row))}))
      (catch Exception e
        nil)))

  (list-all-states [this projection-name]
    (try
      (let [results (jdbc/query db-spec
                                ["SELECT entity_id, state FROM projection_states
                                  WHERE projection_name = ?
                                  ORDER BY updated_at DESC"
                                 (name projection-name)])]
        (map (fn [row]
               [(:entity_id row)
                (json/parse-string (:state row) true)])
             results))
      (catch Exception e
        [])))

  java.io.Closeable
  (close [this]
    (close-connection-pool datasource)))

;; ============================================================================
;; Constructor and Utilities
;; ============================================================================

(defn create-postgres-state-store
  "Create a new PostgreSQL state store.

   Options (all optional):
   - :connection-string - PostgreSQL connection string (default: from env)
   - :username - Database username (default: from env)
   - :password - Database password (default: from env)
   - :pool-size - Connection pool size (default 10)
   - :auto-init-schema - Automatically initialize schema (default true)

   Environment variables:
   - POSTGRES_CONNECTION_STRING
   - POSTGRES_USERNAME
   - POSTGRES_PASSWORD"
  ([]
   (create-postgres-state-store {}))
  ([options]
   (let [connection-string (or (:connection-string options)
                               (System/getenv "POSTGRES_CONNECTION_STRING")
                               "jdbc:postgresql://localhost:5432/allsource")
         username (or (:username options)
                      (System/getenv "POSTGRES_USERNAME")
                      "allsource")
         password (or (:password options)
                      (System/getenv "POSTGRES_PASSWORD")
                      "")
         pool-size (or (:pool-size options) 10)
         auto-init-schema (get options :auto-init-schema true)
         datasource (create-connection-pool
                      {:connection-string connection-string
                       :username username
                       :password password
                       :pool-size pool-size})
         db-spec {:datasource datasource}]
     ;; Initialize schema if requested
     (when auto-init-schema
       (initialize-schema db-spec))
     (->PostgresStateStore datasource db-spec))))

;; ============================================================================
;; Query Utilities
;; ============================================================================

(defn get-projection-statistics
  "Get statistics about a projection in the database.

   Returns:
   - :entity-count - Number of entities
   - :total-size - Total size of state data (approximate)
   - :last-updated - Most recent update timestamp"
  [store projection-name]
  (let [db-spec (:db-spec store)]
    (try
      (let [result (jdbc/query db-spec
                               ["SELECT
                                   COUNT(*) as entity_count,
                                   pg_total_relation_size('projection_states') as total_size,
                                   MAX(updated_at) as last_updated
                                 FROM projection_states
                                 WHERE projection_name = ?"
                                (name projection-name)])]
        (when-let [row (first result)]
          {:entity-count (:entity_count row)
           :total-size (:total_size row)
           :last-updated (:last_updated row)}))
      (catch Exception e
        nil))))

(defn vacuum-old-snapshots
  "Delete snapshots older than a certain date.

   Parameters:
   - store: PostgresStateStore instance
   - older-than: java.time.Instant - Delete snapshots older than this

   Returns: Number of deleted snapshots"
  [store older-than]
  (let [db-spec (:db-spec store)]
    (try
      (let [result (jdbc/execute! db-spec
                                  ["DELETE FROM projection_snapshots
                                    WHERE created_at < ?::timestamp"
                                   (.toString older-than)])]
        (first result))
      (catch Exception e
        0))))

(defn get-state-by-version
  "Get all entities for a specific projection version.

   Useful for auditing and debugging version migrations."
  [store projection-name version]
  (let [db-spec (:db-spec store)]
    (try
      (let [results (jdbc/query db-spec
                                ["SELECT entity_id, state, updated_at
                                  FROM projection_states
                                  WHERE projection_name = ? AND version = ?
                                  ORDER BY updated_at DESC"
                                 (name projection-name)
                                 version])]
        (map (fn [row]
               {:entity-id (:entity_id row)
                :state (json/parse-string (:state row) true)
                :updated-at (:updated_at row)})
             results))
      (catch Exception e
        []))))
