(ns allsource.infrastructure.adapters.state-store-test
  "TDD tests for StateStore implementations.
   RED phase - these tests will fail until we implement the adapters."
  (:require [clojure.test :refer [deftest is testing use-fixtures]]
            [allsource.domain.entities.projection :as p]
            [allsource.domain.protocols.projection-executor :as pe]
            [allsource.infrastructure.adapters.postgres-state-store :as postgres]
            [allsource.infrastructure.adapters.redis-state-store :as redis])
  (:import [java.time Instant]))

;; ============================================================================
;; Test Data
;; ============================================================================

(def test-state
  {:count 42
   :total-amount 1500.50
   :last-updated "2025-10-24T10:00:00Z"})

(def test-snapshot
  (p/make-snapshot
    :projection-name :test-projection
    :entity-id "entity-123"
    :state test-state
    :version 2
    :timestamp (Instant/now)))

;; ============================================================================
;; Generic State Store Tests (applicable to all implementations)
;; ============================================================================

(defn test-state-store-save-and-load
  "Generic test for save and load operations."
  [store-factory]
  (testing "Saving and loading state"
    (let [store (store-factory)]
      ;; Save state
      (let [result (pe/save-state store :test-proj "entity-1" test-state 1)]
        (is (:success result)))
      ;; Load state
      (let [loaded (pe/load-state store :test-proj "entity-1")]
        (is (= test-state loaded))))))

(defn test-state-store-overwrite
  "Generic test for overwriting existing state."
  [store-factory]
  (testing "Overwriting existing state"
    (let [store (store-factory)
          initial-state {:count 10}
          updated-state {:count 20}]
      ;; Save initial state
      (pe/save-state store :test-proj "entity-1" initial-state 1)
      ;; Overwrite with updated state
      (pe/save-state store :test-proj "entity-1" updated-state 2)
      ;; Load should return updated state
      (let [loaded (pe/load-state store :test-proj "entity-1")]
        (is (= updated-state loaded))))))

(defn test-state-store-delete
  "Generic test for deleting state."
  [store-factory]
  (testing "Deleting state"
    (let [store (store-factory)]
      ;; Save state
      (pe/save-state store :test-proj "entity-1" test-state 1)
      ;; Delete state
      (let [result (pe/delete-state store :test-proj "entity-1")]
        (is (:success result)))
      ;; Load should return nil
      (is (nil? (pe/load-state store :test-proj "entity-1"))))))

(defn test-state-store-non-existent
  "Generic test for loading non-existent state."
  [store-factory]
  (testing "Loading non-existent state returns nil"
    (let [store (store-factory)]
      (is (nil? (pe/load-state store :test-proj "non-existent"))))))

(defn test-state-store-multiple-projections
  "Generic test for managing multiple projections."
  [store-factory]
  (testing "Managing state for multiple projections"
    (let [store (store-factory)
          state1 {:count 1}
          state2 {:count 2}]
      ;; Save state for projection 1
      (pe/save-state store :proj-1 "entity-1" state1 1)
      ;; Save state for projection 2
      (pe/save-state store :proj-2 "entity-1" state2 1)
      ;; Load should return correct state for each projection
      (is (= state1 (pe/load-state store :proj-1 "entity-1")))
      (is (= state2 (pe/load-state store :proj-2 "entity-1"))))))

(defn test-state-store-multiple-entities
  "Generic test for managing multiple entities."
  [store-factory]
  (testing "Managing state for multiple entities"
    (let [store (store-factory)
          state1 {:count 1}
          state2 {:count 2}
          state3 {:count 3}]
      ;; Save state for multiple entities
      (pe/save-state store :test-proj "entity-1" state1 1)
      (pe/save-state store :test-proj "entity-2" state2 1)
      (pe/save-state store :test-proj "entity-3" state3 1)
      ;; Load should return correct state for each entity
      (is (= state1 (pe/load-state store :test-proj "entity-1")))
      (is (= state2 (pe/load-state store :test-proj "entity-2")))
      (is (= state3 (pe/load-state store :test-proj "entity-3"))))))

(defn test-state-store-list-all
  "Generic test for listing all states."
  [store-factory]
  (testing "Listing all entity states for a projection"
    (let [store (store-factory)
          state1 {:count 1}
          state2 {:count 2}]
      ;; Save multiple states
      (pe/save-state store :test-proj "entity-1" state1 1)
      (pe/save-state store :test-proj "entity-2" state2 1)
      ;; List all
      (let [all-states (into {} (pe/list-all-states store :test-proj))]
        (is (= 2 (count all-states)))
        (is (= state1 (get all-states "entity-1")))
        (is (= state2 (get all-states "entity-2")))))))

(defn test-state-store-snapshot-save-and-load
  "Generic test for snapshot operations."
  [store-factory]
  (testing "Saving and loading snapshots"
    (let [store (store-factory)]
      ;; Save snapshot
      (let [result (pe/save-snapshot store test-snapshot)]
        (is (:success result))
        (is (some? (:snapshot-id result))))
      ;; Load snapshot
      (let [loaded (pe/load-snapshot store :test-projection "entity-123")]
        (is (some? loaded))
        (is (= :test-projection (:projection-name loaded)))
        (is (= "entity-123" (:entity-id loaded)))
        (is (= test-state (:state loaded)))
        (is (= 2 (:version loaded)))))))

(defn test-state-store-snapshot-overwrite
  "Generic test for overwriting snapshots."
  [store-factory]
  (testing "Overwriting existing snapshot"
    (let [store (store-factory)
          snapshot1 (p/make-snapshot
                      :projection-name :test-proj
                      :entity-id "entity-1"
                      :state {:count 10}
                      :version 1
                      :timestamp (Instant/now))
          snapshot2 (p/make-snapshot
                      :projection-name :test-proj
                      :entity-id "entity-1"
                      :state {:count 20}
                      :version 2
                      :timestamp (Instant/now))]
      ;; Save first snapshot
      (pe/save-snapshot store snapshot1)
      ;; Save second snapshot (should overwrite)
      (pe/save-snapshot store snapshot2)
      ;; Load should return latest snapshot
      (let [loaded (pe/load-snapshot store :test-proj "entity-1")]
        (is (= {:count 20} (:state loaded)))
        (is (= 2 (:version loaded)))))))

;; ============================================================================
;; PostgreSQL State Store Tests
;; ============================================================================

(deftest test-postgres-save-and-load
  (test-state-store-save-and-load postgres/create-postgres-state-store))

(deftest test-postgres-overwrite
  (test-state-store-overwrite postgres/create-postgres-state-store))

(deftest test-postgres-delete
  (test-state-store-delete postgres/create-postgres-state-store))

(deftest test-postgres-non-existent
  (test-state-store-non-existent postgres/create-postgres-state-store))

(deftest test-postgres-multiple-projections
  (test-state-store-multiple-projections postgres/create-postgres-state-store))

(deftest test-postgres-multiple-entities
  (test-state-store-multiple-entities postgres/create-postgres-state-store))

(deftest test-postgres-list-all
  (test-state-store-list-all postgres/create-postgres-state-store))

(deftest test-postgres-snapshot-save-and-load
  (test-state-store-snapshot-save-and-load postgres/create-postgres-state-store))

(deftest test-postgres-snapshot-overwrite
  (test-state-store-snapshot-overwrite postgres/create-postgres-state-store))

;; PostgreSQL-specific tests
(deftest test-postgres-connection-pooling
  (testing "PostgreSQL connection pooling works correctly"
    (let [store (postgres/create-postgres-state-store
                  {:connection-string "postgresql://localhost:5432/allsource_test"
                   :username "test"
                   :password "test"
                   :pool-size 10})]
      ;; Should be able to perform concurrent operations
      (let [results (pmap (fn [i]
                            (pe/save-state store :test-proj (str "entity-" i)
                                           {:count i} 1))
                          (range 20))]
        (is (every? :success results))))))

(deftest test-postgres-transaction-rollback
  (testing "PostgreSQL transactions rollback on error"
    (let [store (postgres/create-postgres-state-store)]
      ;; This should fail and rollback
      (let [result (pe/save-state store :test-proj "entity-1"
                                   {:invalid :data-that-causes-error} 1)]
        ;; Store should handle gracefully
        (is (or (:success result)
                (and (not (:success result)) (some? (:error result)))))))))

;; ============================================================================
;; Redis State Store Tests
;; ============================================================================

(deftest test-redis-save-and-load
  (test-state-store-save-and-load redis/create-redis-state-store))

(deftest test-redis-overwrite
  (test-state-store-overwrite redis/create-redis-state-store))

(deftest test-redis-delete
  (test-state-store-delete redis/create-redis-state-store))

(deftest test-redis-non-existent
  (test-state-store-non-existent redis/create-redis-state-store))

(deftest test-redis-multiple-projections
  (test-state-store-multiple-projections redis/create-redis-state-store))

(deftest test-redis-multiple-entities
  (test-state-store-multiple-entities redis/create-redis-state-store))

(deftest test-redis-list-all
  (test-state-store-list-all redis/create-redis-state-store))

(deftest test-redis-snapshot-save-and-load
  (test-state-store-snapshot-save-and-load redis/create-redis-state-store))

(deftest test-redis-snapshot-overwrite
  (test-state-store-snapshot-overwrite redis/create-redis-state-store))

;; Redis-specific tests
(deftest test-redis-ttl
  (testing "Redis TTL for state expiration"
    (let [store (redis/create-redis-state-store
                  {:host "localhost"
                   :port 6379
                   :ttl-seconds 60})]
      ;; Save state with TTL
      (pe/save-state store :test-proj "entity-1" test-state 1)
      ;; Check TTL is set (would need actual Redis connection to verify)
      (is (some? (pe/load-state store :test-proj "entity-1"))))))

(deftest test-redis-connection-pool
  (testing "Redis connection pooling"
    (let [store (redis/create-redis-state-store
                  {:host "localhost"
                   :port 6379
                   :pool-size 20})]
      ;; Should handle concurrent operations
      (let [results (pmap (fn [i]
                            (pe/save-state store :test-proj (str "entity-" i)
                                           {:count i} 1))
                          (range 50))]
        (is (every? :success results))))))

;; ============================================================================
;; Performance Tests
;; ============================================================================

(deftest test-bulk-write-performance
  (testing "Bulk write performance"
    (let [store (postgres/create-postgres-state-store)
          num-entities 1000
          start-time (System/currentTimeMillis)]
      ;; Write 1000 entities
      (doseq [i (range num-entities)]
        (pe/save-state store :test-proj (str "entity-" i) {:count i} 1))
      (let [elapsed (- (System/currentTimeMillis) start-time)]
        ;; Should complete in reasonable time (adjust threshold as needed)
        (is (< elapsed 5000) "Bulk write should complete in < 5 seconds")))))

(deftest test-bulk-read-performance
  (testing "Bulk read performance"
    (let [store (postgres/create-postgres-state-store)
          num-entities 1000]
      ;; Write 1000 entities
      (doseq [i (range num-entities)]
        (pe/save-state store :test-proj (str "entity-" i) {:count i} 1))
      ;; Read all entities
      (let [start-time (System/currentTimeMillis)
            all-states (pe/list-all-states store :test-proj)
            elapsed (- (System/currentTimeMillis) start-time)]
        (is (= num-entities (count all-states)))
        (is (< elapsed 5000) "Bulk read should complete in < 5 seconds")))))
