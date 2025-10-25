(ns allsource.domain.entities.analytics-test
  "TDD tests for Analytics domain entities.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.analytics :as a]))

;; ============================================================================
;; Aggregation Tests
;; ============================================================================

(deftest test-create-aggregation
  (testing "Creating an aggregation"
    (let [agg (a/make-aggregation :function :sum :field :amount :alias :total-amount)]
      (is (= :sum (:function agg)))
      (is (= :amount (:field agg)))
      (is (= :total-amount (:alias agg))))))

(deftest test-aggregation-function-validation
  (testing "Valid aggregation functions"
    (is (a/valid-aggregation-function? :count))
    (is (a/valid-aggregation-function? :sum))
    (is (a/valid-aggregation-function? :avg))
    (is (a/valid-aggregation-function? :min))
    (is (a/valid-aggregation-function? :max))
    (is (a/valid-aggregation-function? :percentile))
    (is (not (a/valid-aggregation-function? :invalid)))))

(deftest test-aggregation-validation
  (testing "Valid aggregation"
    (let [agg (a/make-aggregation :function :avg :field :amount :alias :avg-amount)]
      (is (a/valid-aggregation? agg))))

  (testing "Invalid aggregation - bad function"
    (let [agg {:function :invalid :field :amount :alias :test}]
      (is (not (a/valid-aggregation? agg))))))

;; ============================================================================
;; Aggregation Factory Tests
;; ============================================================================

(deftest test-count-aggregation
  (testing "Creating count aggregation"
    (let [agg (a/count-aggregation :event-count)]
      (is (= :count (:function agg)))
      (is (= :* (:field agg)))
      (is (= :event-count (:alias agg))))))

(deftest test-sum-aggregation
  (testing "Creating sum aggregation"
    (let [agg (a/sum-aggregation :amount :total)]
      (is (= :sum (:function agg)))
      (is (= :amount (:field agg)))
      (is (= :total (:alias agg))))))

(deftest test-avg-aggregation
  (testing "Creating average aggregation"
    (let [agg (a/avg-aggregation :amount :average)]
      (is (= :avg (:function agg)))
      (is (= :amount (:field agg))))))

(deftest test-percentile-aggregation
  (testing "Creating percentile aggregation"
    (let [agg (a/percentile-aggregation :latency 95 :p95-latency)]
      (is (= :percentile (:function agg)))
      (is (= :latency (:field agg)))
      (is (= 95 (get-in agg [:parameters :percentile]))))))

(deftest test-distinct-count-aggregation
  (testing "Creating distinct count aggregation"
    (let [agg (a/distinct-count-aggregation :user-id :unique-users)]
      (is (= :distinct (:function agg)))
      (is (= :user-id (:field agg))))))

;; ============================================================================
;; Time Series Tests
;; ============================================================================

(deftest test-create-time-series-config
  (testing "Creating time series configuration"
    (let [config (a/make-time-series-config
                   :interval :hour
                   :aggregations [(a/count-aggregation :count)])]
      (is (= :hour (:interval config)))
      (is (= 1 (count (:aggregations config)))))))

(deftest test-time-series-interval-validation
  (testing "Valid time series intervals"
    (is (a/valid-time-series-interval? :second))
    (is (a/valid-time-series-interval? :minute))
    (is (a/valid-time-series-interval? :hour))
    (is (a/valid-time-series-interval? :day))
    (is (a/valid-time-series-interval? :week))
    (is (a/valid-time-series-interval? :month))
    (is (not (a/valid-time-series-interval? :invalid)))))

(deftest test-interval-to-millis
  (testing "Converting intervals to milliseconds"
    (is (= 1000 (a/interval-to-millis :second)))
    (is (= 60000 (a/interval-to-millis :minute)))
    (is (= 3600000 (a/interval-to-millis :hour)))
    (is (= 86400000 (a/interval-to-millis :day)))))

;; ============================================================================
;; Funnel Analysis Tests
;; ============================================================================

(deftest test-create-funnel-step
  (testing "Creating funnel step"
    (let [step (a/make-funnel-step
                 :name :signup
                 :predicate (fn [e] (= "user.signup" (:event-type e)))
                 :order 1)]
      (is (= :signup (:name step)))
      (is (fn? (:predicate step)))
      (is (= 1 (:order step))))))

(deftest test-funnel-step-validation
  (testing "Valid funnel step"
    (let [step (a/make-funnel-step
                 :name :test
                 :predicate (fn [e] true)
                 :order 1)]
      (is (a/valid-funnel-step? step))))

  (testing "Invalid funnel step - invalid order"
    (let [step {:name :test :predicate (fn [e] true) :order 0}]
      (is (not (a/valid-funnel-step? step))))))

(deftest test-create-funnel-config
  (testing "Creating funnel configuration"
    (let [step1 (a/make-funnel-step :name :signup :predicate (fn [e] true) :order 1)
          step2 (a/make-funnel-step :name :activation :predicate (fn [e] true) :order 2)
          config (a/make-funnel-config
                   :name :signup-funnel
                   :steps [step1 step2]
                   :time-window 86400000)]
      (is (= :signup-funnel (:name config)))
      (is (= 2 (count (:steps config))))
      (is (= 86400000 (:time-window config))))))

(deftest test-funnel-steps-sorted-by-order
  (testing "Funnel steps are sorted by order"
    (let [step1 (a/make-funnel-step :name :step1 :predicate (fn [e] true) :order 3)
          step2 (a/make-funnel-step :name :step2 :predicate (fn [e] true) :order 1)
          step3 (a/make-funnel-step :name :step3 :predicate (fn [e] true) :order 2)
          config (a/make-funnel-config
                   :name :test-funnel
                   :steps [step1 step2 step3]
                   :time-window 1000)]
      ;; Should be sorted by order
      (is (= [:step2 :step3 :step1] (map :name (:steps config)))))))

;; ============================================================================
;; Cohort Analysis Tests
;; ============================================================================

(deftest test-create-cohort-config
  (testing "Creating cohort configuration"
    (let [config (a/make-cohort-config
                   :name :monthly-cohorts
                   :cohort-fn (fn [e] (get-in e [:created-at :month]))
                   :time-interval :month
                   :metric-fn (fn [events] (count events)))]
      (is (= :monthly-cohorts (:name config)))
      (is (fn? (:cohort-fn config)))
      (is (= :month (:time-interval config)))
      (is (= 12 (:retention-periods config))))))

;; ============================================================================
;; Trend Analysis Tests
;; ============================================================================

(deftest test-create-trend-config
  (testing "Creating trend configuration"
    (let [config (a/make-trend-config
                   :metric-name :daily-revenue
                   :time-interval :day
                   :smoothing :moving-average
                   :window-size 7)]
      (is (= :daily-revenue (:metric-name config)))
      (is (= :day (:time-interval config)))
      (is (= :moving-average (:smoothing config)))
      (is (= 7 (:window-size config))))))

(deftest test-trend-config-defaults
  (testing "Trend configuration defaults"
    (let [config (a/make-trend-config
                   :metric-name :test
                   :time-interval :hour)]
      (is (= :moving-average (:smoothing config)))
      (is (= 7 (:window-size config))))))

;; ============================================================================
;; Anomaly Detection Tests
;; ============================================================================

(deftest test-create-anomaly-config
  (testing "Creating anomaly detection configuration"
    (let [config (a/make-anomaly-config
                   :metric-name :request-latency
                   :algorithm :zscore
                   :sensitivity 3
                   :baseline-window 30)]
      (is (= :request-latency (:metric-name config)))
      (is (= :zscore (:algorithm config)))
      (is (= 3 (:sensitivity config)))
      (is (= 30 (:baseline-window config))))))

(deftest test-anomaly-config-defaults
  (testing "Anomaly configuration defaults"
    (let [config (a/make-anomaly-config :metric-name :test)]
      (is (= :zscore (:algorithm config)))
      (is (= 3 (:sensitivity config)))
      (is (= 30 (:baseline-window config))))))

;; ============================================================================
;; Time Grouping Tests
;; ============================================================================

(def test-events
  [{:event-type "order" :timestamp 1000 :amount 100}
   {:event-type "order" :timestamp 2000 :amount 150}
   {:event-type "order" :timestamp 61000 :amount 200}
   {:event-type "order" :timestamp 62000 :amount 250}])

(deftest test-group-by-interval
  (testing "Grouping events by time interval"
    (let [grouped (a/group-by-interval test-events :minute :timestamp)]
      ;; Should have 2 groups: 0-60000 and 60000-120000
      (is (= 2 (count grouped)))
      (is (= 2 (count (get grouped 0))))  ; First minute: 2 events
      (is (= 2 (count (get grouped 60000)))))))  ; Second minute: 2 events
