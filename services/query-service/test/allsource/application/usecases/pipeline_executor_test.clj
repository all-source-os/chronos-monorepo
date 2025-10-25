(ns allsource.application.usecases.pipeline-executor-test
  "TDD tests for Pipeline Executor implementation.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.pipeline :as p]
            [allsource.application.usecases.pipeline-executor :as exec]))

;; ============================================================================
;; Test Data
;; ============================================================================

(def test-events
  [{:event-type "user.created" :entity-id "user-1" :timestamp 1000 :payload {:name "Alice"}}
   {:event-type "user.created" :entity-id "user-2" :timestamp 2000 :payload {:name "Bob"}}
   {:event-type "order.placed" :entity-id "user-1" :timestamp 3000 :payload {:amount 100}}
   {:event-type "user.created" :entity-id "user-3" :timestamp 4000 :payload {:name "Charlie"}}
   {:event-type "order.placed" :entity-id "user-2" :timestamp 5000 :payload {:amount 200}}])

;; ============================================================================
;; Filter Operator Tests
;; ============================================================================

(deftest test-filter-operator-execution
  (testing "Executing filter operator"
    (let [filter-op (p/filter-operator :user-events
                                        (fn [e] (= "user.created" (:event-type e))))
          result (exec/apply-operator filter-op test-events)]
      (is (= 3 (count result)))
      (is (every? #(= "user.created" (:event-type %)) result)))))

(deftest test-filter-operator-empty-result
  (testing "Filter that matches no events"
    (let [filter-op (p/filter-operator :non-matching
                                        (fn [e] (= "non-existent" (:event-type e))))
          result (exec/apply-operator filter-op test-events)]
      (is (empty? result)))))

;; ============================================================================
;; Map Operator Tests
;; ============================================================================

(deftest test-map-operator-execution
  (testing "Executing map operator"
    (let [map-op (p/map-operator :add-flag
                                  (fn [e] (assoc e :processed true)))
          result (exec/apply-operator map-op test-events)]
      (is (= (count test-events) (count result)))
      (is (every? :processed result)))))

(deftest test-map-operator-transformation
  (testing "Transforming event structure"
    (let [map-op (p/map-operator :extract-type
                                  (fn [e] {:type (:event-type e)}))
          result (exec/apply-operator map-op test-events)]
      (is (every? #(contains? % :type) result))
      (is (every? #(not (contains? % :payload)) result)))))

;; ============================================================================
;; Enrich Operator Tests
;; ============================================================================

(deftest test-enrich-operator-execution
  (testing "Executing enrich operator"
    (let [enrich-op (p/enrich-operator :add-metadata
                                        (fn [e] (assoc e :enriched-at (System/currentTimeMillis))))
          result (exec/apply-operator enrich-op test-events)]
      (is (= (count test-events) (count result)))
      (is (every? :enriched-at result)))))

;; ============================================================================
;; Pipeline Execution Tests
;; ============================================================================

(deftest test-single-operator-pipeline
  (testing "Executing pipeline with single operator"
    (let [filter-op (p/filter-operator :user-events
                                        (fn [e] (= "user.created" (:event-type e))))
          pipeline (p/make-pipeline :name :test-pipeline
                                    :version 1
                                    :operators [filter-op])
          result (exec/execute-pipeline pipeline test-events)]
      (is (= 3 (count result)))
      (is (every? #(= "user.created" (:event-type %)) result)))))

(deftest test-multi-operator-pipeline
  (testing "Executing pipeline with multiple operators"
    (let [filter-op (p/filter-operator :user-events
                                        (fn [e] (= "user.created" (:event-type e))))
          map-op (p/map-operator :extract-name
                                  (fn [e] {:name (get-in e [:payload :name])}))
          pipeline (p/make-pipeline :name :test-pipeline
                                    :version 1
                                    :operators [filter-op map-op])
          result (exec/execute-pipeline pipeline test-events)]
      (is (= 3 (count result)))
      (is (every? :name result))
      (is (= #{"Alice" "Bob" "Charlie"} (set (map :name result)))))))

(deftest test-empty-pipeline
  (testing "Executing pipeline with no operators"
    (let [pipeline (p/make-pipeline :name :empty :version 1 :operators [])
          result (exec/execute-pipeline pipeline test-events)]
      (is (= test-events result)))))

;; ============================================================================
;; Batch Operator Tests
;; ============================================================================

(deftest test-batch-operator-execution
  (testing "Executing batch operator"
    (let [batch-op (p/batch-operator :batch-2 2 (fn [batch] (count batch)))
          result (exec/apply-operator batch-op test-events)]
      ;; Should produce batches: [2, 2, 1] => [2, 2, 1]
      (is (= 3 (count result)))
      (is (= [2 2 1] result)))))

(deftest test-batch-operator-full-batches
  (testing "Batching with exact division"
    (let [batch-op (p/batch-operator :batch-1 1 identity)
          result (exec/apply-operator batch-op (take 4 test-events))]
      (is (= 4 (count result))))))

;; ============================================================================
;; Window Operator Tests
;; ============================================================================

(deftest test-tumbling-window
  (testing "Tumbling window operator"
    (let [window-config (p/make-window-config :type :tumbling :size 2000)
          window-op (p/window-operator :time-window window-config count)
          ;; Events at timestamps: 1000, 2000, 3000, 4000, 5000
          ;; Windows: [0-2000]: 2 events, [2000-4000]: 2 events, [4000-6000]: 1 event
          result (exec/apply-operator window-op test-events)]
      ;; Should produce 3 windows with counts [2, 2, 1]
      (is (= 3 (count result)))
      (is (= [2 2 1] result)))))

(deftest test-sliding-window
  (testing "Sliding window operator"
    (let [window-config (p/make-window-config :type :sliding :size 3000 :slide 1000)
          window-op (p/window-operator :sliding-window window-config count)
          result (exec/apply-operator window-op test-events)]
      ;; Sliding windows will overlap, producing more windows
      (is (pos? (count result))))))

;; ============================================================================
;; Error Handling Tests
;; ============================================================================

(deftest test-operator-error-handling
  (testing "Handling errors in operator function"
    (let [bad-op (p/map-operator :error-op
                                  (fn [e] (throw (ex-info "Test error" {}))))
          result (exec/apply-operator-safe bad-op test-events)]
      ;; Should handle error gracefully
      (is (map? result))
      (is (= :error (:status result)))
      (is (some? (:error result))))))

(deftest test-pipeline-partial-failure
  (testing "Pipeline continues on operator failure with error handling"
    (let [good-op (p/filter-operator :user-events
                                      (fn [e] (= "user.created" (:event-type e))))
          bad-op (p/map-operator :error-op
                                  (fn [e] (throw (ex-info "Test error" {}))))
          pipeline (p/make-pipeline :name :test :version 1 :operators [good-op bad-op])
          result (exec/execute-pipeline-safe pipeline test-events)]
      (is (or (= :error (:status result))
              (empty? result))))))

;; ============================================================================
;; Async Pipeline Execution Tests
;; ============================================================================

(deftest test-async-pipeline-execution
  (testing "Asynchronous pipeline execution"
    (let [filter-op (p/filter-operator :user-events
                                        (fn [e] (= "user.created" (:event-type e))))
          pipeline (p/make-pipeline :name :async-test :version 1 :operators [filter-op])
          result-promise (promise)
          callback (fn [result] (deliver result-promise result))]
      (exec/execute-pipeline-async pipeline test-events callback)
      ;; Wait for result
      (let [result (deref result-promise 5000 :timeout)]
        (is (not= :timeout result))
        (is (= 3 (count result)))))))

;; ============================================================================
;; Pipeline Metrics Tests
;; ============================================================================

(deftest test-pipeline-metrics-collection
  (testing "Collecting metrics during pipeline execution"
    (let [filter-op (p/filter-operator :user-events
                                        (fn [e] (= "user.created" (:event-type e))))
          pipeline (p/make-pipeline :name :metrics-test :version 1 :operators [filter-op])
          executor (exec/create-executor)
          result (exec/execute-with-metrics executor pipeline test-events)
          metrics (:metrics result)]
      (is (some? metrics))
      (is (= 5 (:total-processed metrics)))  ; 5 input events
      (is (= 2 (:total-filtered metrics)))   ; 2 order events filtered out
      (is (pos? (:throughput metrics))))))

(deftest test-operator-level-metrics
  (testing "Collecting metrics per operator"
    (let [op1 (p/filter-operator :filter1 (fn [e] true))
          op2 (p/map-operator :map1 identity)
          pipeline (p/make-pipeline :name :test :version 1 :operators [op1 op2])
          executor (exec/create-executor)
          result (exec/execute-with-metrics executor pipeline test-events)
          op-metrics (:operator-metrics result)]
      (is (some? op-metrics))
      (is (= 2 (count op-metrics)))
      (is (contains? op-metrics :filter1))
      (is (contains? op-metrics :map1)))))

;; ============================================================================
;; Backpressure Tests
;; ============================================================================

(deftest test-backpressure-drop-strategy
  (testing "Drop strategy drops events when buffer is full"
    (let [config (p/make-backpressure-config :strategy :drop :buffer-size 2)
          executor (exec/create-executor :backpressure-config config)
          slow-op (p/map-operator :slow (fn [e] (Thread/sleep 100) e))
          pipeline (p/make-pipeline :name :test :version 1 :operators [slow-op])
          ;; Send more events than buffer can handle
          result (exec/execute-with-backpressure executor pipeline test-events)]
      ;; Some events should be dropped
      (is (<= (count result) (count test-events))))))

(deftest test-backpressure-buffer-strategy
  (testing "Buffer strategy buffers events"
    (let [config (p/make-backpressure-config :strategy :buffer :buffer-size 100)
          executor (exec/create-executor :backpressure-config config)
          pipeline (p/make-pipeline :name :test :version 1 :operators [])
          result (exec/execute-with-backpressure executor pipeline test-events)]
      ;; All events should be processed
      (is (= (count test-events) (count result))))))

(deftest test-backpressure-block-strategy
  (testing "Block strategy blocks when buffer is full"
    (let [config (p/make-backpressure-config :strategy :block
                                             :buffer-size 2
                                             :timeout-ms 1000)
          executor (exec/create-executor :backpressure-config config)
          slow-op (p/map-operator :slow (fn [e] (Thread/sleep 50) e))
          pipeline (p/make-pipeline :name :test :version 1 :operators [slow-op])
          start-time (System/currentTimeMillis)
          result (exec/execute-with-backpressure executor pipeline test-events)
          elapsed (- (System/currentTimeMillis) start-time)]
      ;; Should take some time due to blocking
      (is (pos? elapsed)))))

;; ============================================================================
;; Stateful Operator Tests
;; ============================================================================

(deftest test-stateful-operator
  (testing "Operator with internal state"
    (let [counter (atom 0)
          stateful-op (p/map-operator :counter
                                       (fn [e]
                                         (assoc e :index (swap! counter inc))))
          result (exec/apply-operator stateful-op test-events)]
      (is (= 5 (count result)))
      (is (= [1 2 3 4 5] (map :index result))))))

;; ============================================================================
;; Parallel Execution Tests
;; ============================================================================

(deftest test-parallel-pipeline-execution
  (testing "Executing pipeline in parallel"
    (let [map-op (p/map-operator :add-thread-id
                                  (fn [e] (assoc e :thread (.getName (Thread/currentThread)))))
          pipeline (p/make-pipeline :name :parallel :version 1 :operators [map-op])
          executor (exec/create-executor :parallel true :parallelism 4)
          result (exec/execute-pipeline executor pipeline test-events)]
      ;; Should process all events
      (is (= (count test-events) (count result)))
      ;; May use multiple threads
      (let [threads (set (map :thread result))]
        (is (pos? (count threads)))))))
