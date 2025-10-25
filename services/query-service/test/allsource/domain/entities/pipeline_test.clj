(ns allsource.domain.entities.pipeline-test
  "TDD tests for Pipeline domain entities.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.pipeline :as p]))

;; ============================================================================
;; Operator Creation Tests
;; ============================================================================

(deftest test-create-pipeline-operator
  (testing "Creating a pipeline operator"
    (let [op (p/make-operator
               :type :filter
               :name :test-filter
               :operator-fn (fn [event] (= "test" (:event-type event))))]
      (is (= :filter (:type op)))
      (is (= :test-filter (:name op)))
      (is (fn? (:operator-fn op))))))

(deftest test-operator-type-validation
  (testing "Valid operator types"
    (is (p/valid-operator-type? :filter))
    (is (p/valid-operator-type? :map))
    (is (p/valid-operator-type? :enrich))
    (is (p/valid-operator-type? :window))
    (is (not (p/valid-operator-type? :invalid)))))

(deftest test-operator-validation
  (testing "Valid operator"
    (let [valid-op (p/make-operator
                     :type :filter
                     :name :test
                     :operator-fn (fn [e] true))]
      (is (p/valid-operator? valid-op))))

  (testing "Invalid operator - bad type"
    (let [invalid-op {:type :invalid :name :test :operator-fn (fn [e] true)}]
      (is (not (p/valid-operator? invalid-op))))))

;; ============================================================================
;; Pipeline Creation Tests
;; ============================================================================

(deftest test-create-pipeline
  (testing "Creating a pipeline"
    (let [pipeline (p/make-pipeline
                     :name :test-pipeline
                     :version 1
                     :operators [])]
      (is (= :test-pipeline (:name pipeline)))
      (is (= 1 (:version pipeline)))
      (is (vector? (:operators pipeline))))))

(deftest test-pipeline-validation
  (testing "Valid pipeline"
    (let [op (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          pipeline (p/make-pipeline
                     :name :test
                     :version 1
                     :operators [op])]
      (is (p/valid-pipeline? pipeline))))

  (testing "Invalid pipeline - bad version"
    (let [pipeline (p/make-pipeline
                     :name :test
                     :version -1
                     :operators [])]
      (is (not (p/valid-pipeline? pipeline))))))

;; ============================================================================
;; Pipeline Composition Tests
;; ============================================================================

(deftest test-add-operator
  (testing "Adding operator to pipeline"
    (let [pipeline (p/make-pipeline :name :test :version 1 :operators [])
          op (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          updated (p/add-operator pipeline op)]
      (is (= 1 (count (:operators updated))))
      (is (= op (first (:operators updated)))))))

(deftest test-remove-operator
  (testing "Removing operator from pipeline"
    (let [op1 (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          op2 (p/make-operator :type :map :name :op2 :operator-fn identity)
          pipeline (p/make-pipeline :name :test :version 1 :operators [op1 op2])
          updated (p/remove-operator pipeline :op1)]
      (is (= 1 (count (:operators updated))))
      (is (= :op2 (:name (first (:operators updated))))))))

(deftest test-replace-operator
  (testing "Replacing operator in pipeline"
    (let [op1 (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          op2 (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] false))
          pipeline (p/make-pipeline :name :test :version 1 :operators [op1])
          updated (p/replace-operator pipeline :op1 op2)]
      (is (= 1 (count (:operators updated))))
      (is (= op2 (first (:operators updated)))))))

;; ============================================================================
;; Operator Factory Tests
;; ============================================================================

(deftest test-filter-operator
  (testing "Creating filter operator"
    (let [op (p/filter-operator :user-events
                                 (fn [e] (= "user.created" (:event-type e))))]
      (is (= :filter (:type op)))
      (is (= :user-events (:name op)))
      (is (fn? (:operator-fn op))))))

(deftest test-map-operator
  (testing "Creating map operator"
    (let [op (p/map-operator :add-timestamp
                              (fn [e] (assoc e :processed-at (System/currentTimeMillis))))]
      (is (= :map (:type op)))
      (is (= :add-timestamp (:name op)))
      (is (fn? (:operator-fn op))))))

(deftest test-enrich-operator
  (testing "Creating enrich operator"
    (let [op (p/enrich-operator :add-user-data
                                 (fn [e] (assoc e :user-data {:name "Test"})))]
      (is (= :enrich (:type op)))
      (is (= :add-user-data (:name op)))
      (is (fn? (:operator-fn op))))))

(deftest test-window-operator
  (testing "Creating window operator"
    (let [window-config (p/make-window-config :type :tumbling :size 1000)
          op (p/window-operator :count-window window-config count)]
      (is (= :window (:type op)))
      (is (= :count-window (:name op)))
      (is (fn? (:operator-fn op)))
      (is (= window-config (get-in op [:config :window]))))))

(deftest test-batch-operator
  (testing "Creating batch operator"
    (let [op (p/batch-operator :batch-10 10 (fn [batch] (count batch)))]
      (is (= :batch (:type op)))
      (is (= :batch-10 (:name op)))
      (is (= 10 (get-in op [:config :batch-size]))))))

;; ============================================================================
;; Window Configuration Tests
;; ============================================================================

(deftest test-tumbling-window-config
  (testing "Creating tumbling window configuration"
    (let [config (p/make-window-config :type :tumbling :size 1000)]
      (is (= :tumbling (:type config)))
      (is (= 1000 (:size config))))))

(deftest test-sliding-window-config
  (testing "Creating sliding window configuration"
    (let [config (p/make-window-config :type :sliding :size 1000 :slide 500)]
      (is (= :sliding (:type config)))
      (is (= 1000 (:size config)))
      (is (= 500 (:slide config))))))

(deftest test-session-window-config
  (testing "Creating session window configuration"
    (let [config (p/make-window-config :type :session :timeout 5000)]
      (is (= :session (:type config)))
      (is (= 5000 (:timeout config))))))

;; ============================================================================
;; Backpressure Configuration Tests
;; ============================================================================

(deftest test-backpressure-drop-strategy
  (testing "Creating drop backpressure configuration"
    (let [config (p/make-backpressure-config :strategy :drop)]
      (is (= :drop (:strategy config))))))

(deftest test-backpressure-buffer-strategy
  (testing "Creating buffer backpressure configuration"
    (let [config (p/make-backpressure-config :strategy :buffer :buffer-size 5000)]
      (is (= :buffer (:strategy config)))
      (is (= 5000 (:buffer-size config))))))

(deftest test-backpressure-block-strategy
  (testing "Creating block backpressure configuration"
    (let [config (p/make-backpressure-config :strategy :block :timeout-ms 10000)]
      (is (= :block (:strategy config)))
      (is (= 10000 (:timeout-ms config))))))

;; ============================================================================
;; Accessor Tests
;; ============================================================================

(deftest test-get-pipeline-name
  (testing "Getting pipeline name"
    (let [pipeline (p/make-pipeline :name :test-pipeline :version 1 :operators [])]
      (is (= :test-pipeline (p/get-name pipeline))))))

(deftest test-get-pipeline-version
  (testing "Getting pipeline version"
    (let [pipeline (p/make-pipeline :name :test :version 3 :operators [])]
      (is (= 3 (p/get-version pipeline))))))

(deftest test-get-operators
  (testing "Getting pipeline operators"
    (let [op (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          pipeline (p/make-pipeline :name :test :version 1 :operators [op])]
      (is (= [op] (p/get-operators pipeline))))))

(deftest test-get-operator-by-name
  (testing "Getting operator by name"
    (let [op1 (p/make-operator :type :filter :name :op1 :operator-fn (fn [e] true))
          op2 (p/make-operator :type :map :name :op2 :operator-fn identity)
          pipeline (p/make-pipeline :name :test :version 1 :operators [op1 op2])]
      (is (= op2 (p/get-operator-by-name pipeline :op2)))
      (is (nil? (p/get-operator-by-name pipeline :non-existent))))))

;; ============================================================================
;; Metrics Tests
;; ============================================================================

(deftest test-create-metrics
  (testing "Creating pipeline metrics"
    (let [metrics (p/make-metrics
                    :pipeline-name :test
                    :total-processed 1000
                    :total-filtered 100
                    :throughput 50.5)]
      (is (= :test (:pipeline-name metrics)))
      (is (= 1000 (:total-processed metrics)))
      (is (= 100 (:total-filtered metrics)))
      (is (= 50.5 (:throughput metrics))))))
