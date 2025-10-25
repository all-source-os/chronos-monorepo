(ns allsource.application.usecases.projection-executor-test
  "TDD tests for ProjectionExecutor implementation.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing use-fixtures]]
            [allsource.domain.entities.projection :as p]
            [allsource.domain.protocols.projection-executor :as pe]
            [allsource.application.usecases.projection-executor :as exec]))

;; ============================================================================
;; Test Fixtures
;; ============================================================================

(def test-projection
  "Simple projection for testing - counts user creation events."
  (p/make-projection
    :name :test-user-count
    :version 1
    :initial-state {:count 0}
    :project-fn (fn [state event]
                  (if (= "user.created" (:event-type event))
                    (update state :count inc)
                    state))))

(def test-events
  [{:event-type "user.created" :entity-id "user-1" :timestamp 1000}
   {:event-type "user.created" :entity-id "user-2" :timestamp 2000}
   {:event-type "order.placed" :entity-id "user-1" :timestamp 3000}
   {:event-type "user.created" :entity-id "user-3" :timestamp 4000}])

;; ============================================================================
;; Projection Executor Tests
;; ============================================================================

(deftest test-start-projection
  (testing "Starting a projection successfully"
    (let [executor (exec/create-executor)
          result (pe/start-projection executor test-projection)]
      (is (= :started (:status result)))
      (is (= :test-user-count (:projection-name result)))))

  (testing "Cannot start projection with same name twice"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (let [result (pe/start-projection executor test-projection)]
        (is (= :error (:status result)))
        (is (some? (:error result)))))))

(deftest test-stop-projection
  (testing "Stopping a running projection"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (let [result (pe/stop-projection executor :test-user-count)]
        (is (= :stopped (:status result)))
        (is (= :test-user-count (:projection-name result))))))

  (testing "Stopping a non-existent projection returns error"
    (let [executor (exec/create-executor)
          result (pe/stop-projection executor :non-existent)]
      (is (= :error (:status result))))))

(deftest test-reload-projection
  (testing "Reloading a projection with new definition"
    (let [executor (exec/create-executor)
          updated-projection (p/make-projection
                               :name :test-user-count
                               :version 2
                               :initial-state {:count 0 :new-field true}
                               :project-fn (fn [state event] state))]
      (pe/start-projection executor test-projection)
      (let [result (pe/reload-projection executor updated-projection)]
        (is (= :reloaded (:status result)))
        (is (= :test-user-count (:projection-name result)))
        ;; Verify new version is active
        (let [status (pe/get-projection-status executor :test-user-count)]
          (is (= 2 (:version status)))))))

  (testing "Reloading non-existent projection starts it"
    (let [executor (exec/create-executor)
          result (pe/reload-projection executor test-projection)]
      (is (or (= :reloaded (:status result))
              (= :started (:status result)))))))

(deftest test-get-projection-state
  (testing "Getting projection state for an entity"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      ;; Process events manually (will be automated in real implementation)
      (exec/process-event executor :test-user-count (first test-events))
      (let [state (pe/get-projection-state executor :test-user-count "user-1")]
        (is (some? state))
        (is (= 1 (:count state))))))

  (testing "Getting state for non-existent entity returns nil"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (is (nil? (pe/get-projection-state executor :test-user-count "unknown")))))

  (testing "Getting state for stopped projection returns error"
    (let [executor (exec/create-executor)]
      (is (nil? (pe/get-projection-state executor :test-user-count "user-1"))))))

(deftest test-get-projection-status
  (testing "Getting status of running projection"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (let [status (pe/get-projection-status executor :test-user-count)]
        (is (= :running (:status status)))
        (is (= 1 (:version status)))
        (is (number? (:entities-count status))))))

  (testing "Getting status of stopped projection"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (pe/stop-projection executor :test-user-count)
      (let [status (pe/get-projection-status executor :test-user-count)]
        (is (or (= :stopped (:status status))
                (nil? status))))))

  (testing "Getting status of non-existent projection returns nil"
    (let [executor (exec/create-executor)
          status (pe/get-projection-status executor :non-existent)]
      (is (nil? status)))))

;; ============================================================================
;; Event Processing Tests
;; ============================================================================

(deftest test-process-single-event
  (testing "Processing a single event updates state"
    (let [executor (exec/create-executor)
          event {:event-type "user.created" :entity-id "user-1" :timestamp 1000}]
      (pe/start-projection executor test-projection)
      (exec/process-event executor :test-user-count event)
      (let [state (pe/get-projection-state executor :test-user-count "user-1")]
        (is (= 1 (:count state)))))))

(deftest test-process-multiple-events
  (testing "Processing multiple events accumulates state"
    (let [executor (exec/create-executor)]
      (pe/start-projection executor test-projection)
      (doseq [event test-events]
        (exec/process-event executor :test-user-count event))
      ;; Should have 3 user.created events
      (let [state (pe/get-projection-state executor :test-user-count "global")]
        (is (= 3 (:count state)))))))

(deftest test-event-processing-immutability
  (testing "Event processing doesn't modify original state"
    (let [executor (exec/create-executor)
          event {:event-type "user.created" :entity-id "user-1"}]
      (pe/start-projection executor test-projection)
      (let [state-before (pe/get-projection-state executor :test-user-count "user-1")]
        (exec/process-event executor :test-user-count event)
        ;; Original state reference should be unchanged if it existed
        (when state-before
          (is (not= state-before
                    (pe/get-projection-state executor :test-user-count "user-1"))))))))

;; ============================================================================
;; State Management Tests
;; ============================================================================

(deftest test-projection-state-persistence
  (testing "Projection state persists across restarts"
    (let [executor (exec/create-executor)
          event {:event-type "user.created" :entity-id "user-1"}]
      (pe/start-projection executor test-projection)
      (exec/process-event executor :test-user-count event)
      (let [state-before (pe/get-projection-state executor :test-user-count "user-1")]
        ;; Stop and restart
        (pe/stop-projection executor :test-user-count)
        (pe/start-projection executor test-projection)
        (let [state-after (pe/get-projection-state executor :test-user-count "user-1")]
          (is (= state-before state-after)))))))

(deftest test-snapshot-creation
  (testing "Creating snapshots of projection state"
    (let [executor (exec/create-executor)
          events (take 100 test-events)]
      (pe/start-projection executor test-projection)
      (doseq [event events]
        (exec/process-event executor :test-user-count event))
      ;; Trigger snapshot
      (let [result (exec/create-snapshot executor :test-user-count "user-1")]
        (is (:success result))
        (is (some? (:snapshot-id result)))))))

(deftest test-restore-from-snapshot
  (testing "Restoring projection state from snapshot"
    (let [executor (exec/create-executor)
          events (take 100 test-events)]
      (pe/start-projection executor test-projection)
      (doseq [event events]
        (exec/process-event executor :test-user-count event))
      (let [state-before (pe/get-projection-state executor :test-user-count "user-1")]
        (exec/create-snapshot executor :test-user-count "user-1")
        ;; Clear state
        (pe/stop-projection executor :test-user-count)
        (pe/start-projection executor test-projection)
        ;; Restore from snapshot
        (exec/restore-from-snapshot executor :test-user-count "user-1")
        (let [state-after (pe/get-projection-state executor :test-user-count "user-1")]
          (is (= state-before state-after)))))))

;; ============================================================================
;; Multi-Projection Tests
;; ============================================================================

(deftest test-multiple-projections
  (testing "Running multiple projections simultaneously"
    (let [executor (exec/create-executor)
          projection-1 test-projection
          projection-2 (p/make-projection
                         :name :test-order-count
                         :version 1
                         :initial-state {:count 0}
                         :project-fn (fn [state event]
                                       (if (= "order.placed" (:event-type event))
                                         (update state :count inc)
                                         state)))]
      (pe/start-projection executor projection-1)
      (pe/start-projection executor projection-2)
      (doseq [event test-events]
        (exec/process-event executor :test-user-count event)
        (exec/process-event executor :test-order-count event))
      (let [user-state (pe/get-projection-state executor :test-user-count "global")
            order-state (pe/get-projection-state executor :test-order-count "global")]
        (is (= 3 (:count user-state)))  ; 3 user.created events
        (is (= 1 (:count order-state)))))))  ; 1 order.placed event

;; ============================================================================
;; Error Handling Tests
;; ============================================================================

(deftest test-projection-function-error
  (testing "Handling errors in projection function"
    (let [executor (exec/create-executor)
          bad-projection (p/make-projection
                           :name :bad-projection
                           :version 1
                           :initial-state {}
                           :project-fn (fn [state event]
                                         (throw (ex-info "Projection error" {}))))]
      (pe/start-projection executor bad-projection)
      (let [result (exec/process-event executor :bad-projection {:event-type "test"})]
        (is (= :error (:status result)))
        (is (some? (:error result)))))))

(deftest test-invalid-projection-definition
  (testing "Starting with invalid projection definition"
    (let [executor (exec/create-executor)
          invalid-projection (p/make-projection
                               :name :invalid
                               :version -1  ; Invalid version
                               :initial-state {}
                               :project-fn (fn [s e] s))]
      (let [result (pe/start-projection executor invalid-projection)]
        (is (= :error (:status result)))
        (is (some? (:error result)))))))
