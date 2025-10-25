(ns allsource.domain.entities.integration-test
  "TDD tests for Integration domain entities.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.integration :as i])
  (:import [java.time Instant]))

;; ============================================================================
;; Replay Configuration Tests
;; ============================================================================

(deftest test-create-replay-config
  (testing "Creating replay configuration"
    (let [config (i/make-replay-config
                   :name :test-replay
                   :start-time (Instant/parse "2025-01-01T00:00:00Z")
                   :end-time (Instant/now)
                   :target :my-projection)]
      (is (= :test-replay (:name config)))
      (is (some? (:start-time config)))
      (is (some? (:end-time config)))
      (is (= :my-projection (:target config))))))

(deftest test-replay-speed-validation
  (testing "Valid replay speeds"
    (is (i/valid-replay-speed? 0))
    (is (i/valid-replay-speed? 1.0))
    (is (i/valid-replay-speed? 2.5))
    (is (not (i/valid-replay-speed? -1)))))

(deftest test-replay-config-defaults
  (testing "Replay configuration defaults"
    (let [config (i/make-replay-config
                   :name :test
                   :start-time (Instant/now)
                   :end-time (Instant/now))]
      (is (= 0 (:speed config)))  ; max speed
      (is (= 1000 (:batch-size config)))
      (is (= false (:parallel config))))))

;; ============================================================================
;; Validation Rule Tests
;; ============================================================================

(deftest test-create-validation-rule
  (testing "Creating validation rule"
    (let [rule (i/make-validation-rule
                 :name :has-timestamp
                 :predicate (fn [e] (some? (:timestamp e)))
                 :severity :error
                 :message-fn (fn [e] "Missing timestamp"))]
      (is (= :has-timestamp (:name rule)))
      (is (fn? (:predicate rule)))
      (is (= :error (:severity rule)))
      (is (= true (:enabled rule))))))

(deftest test-validation-severity-validation
  (testing "Valid validation severities"
    (is (i/valid-validation-severity? :error))
    (is (i/valid-validation-severity? :warning))
    (is (i/valid-validation-severity? :info))
    (is (not (i/valid-validation-severity? :invalid)))))

(deftest test-validation-rule-validation
  (testing "Valid validation rule"
    (let [rule (i/make-validation-rule
                 :name :test
                 :predicate (fn [e] true)
                 :severity :error
                 :message-fn (fn [e] "error"))]
      (is (i/valid-validation-rule? rule)))))

;; ============================================================================
;; Validation Config Tests
;; ============================================================================

(deftest test-create-validation-config
  (testing "Creating validation configuration"
    (let [rule1 (i/make-validation-rule
                  :name :rule1
                  :predicate (fn [e] true)
                  :severity :error
                  :message-fn (fn [e] "error"))
          config (i/make-validation-config
                   :name :test-validation
                   :rules [rule1])]
      (is (= :test-validation (:name config)))
      (is (= 1 (count (:rules config))))
      (is (= false (:fail-fast config)))
      (is (= true (:collect-all config))))))

;; ============================================================================
;; Common Validation Rules Tests
;; ============================================================================

(deftest test-required-field-rule
  (testing "Required field validation rule"
    (let [rule (i/required-field-rule :timestamp :error)
          event-with-field {:timestamp 1000 :data "test"}
          event-without-field {:data "test"}]
      (is ((:predicate rule) event-with-field))
      (is (not ((:predicate rule) event-without-field))))))

(deftest test-field-type-rule
  (testing "Field type validation rule"
    (let [rule (i/field-type-rule :amount :number :error)
          valid-event {:amount 100}
          invalid-event {:amount "not-a-number"}]
      (is ((:predicate rule) valid-event))
      (is (not ((:predicate rule) invalid-event))))))

(deftest test-field-range-rule
  (testing "Field range validation rule"
    (let [rule (i/field-range-rule :age 0 120 :warning)
          valid-event {:age 25}
          invalid-low {:age -5}
          invalid-high {:age 150}]
      (is ((:predicate rule) valid-event))
      (is (not ((:predicate rule) invalid-low)))
      (is (not ((:predicate rule) invalid-high))))))

(deftest test-event-type-rule
  (testing "Event type validation rule"
    (let [rule (i/event-type-rule ["user.created" "user.updated"] :error)
          valid-event {:event-type "user.created"}
          invalid-event {:event-type "order.placed"}]
      (is ((:predicate rule) valid-event))
      (is (not ((:predicate rule) invalid-event))))))

;; ============================================================================
;; Schema Tests
;; ============================================================================

(deftest test-create-schema
  (testing "Creating schema"
    (let [schema (i/make-schema
                   :name :user-event-schema
                   :version 1
                   :fields {:user-id {:type :string}
                            :timestamp {:type :number}}
                   :required [:user-id :timestamp])]
      (is (= :user-event-schema (:name schema)))
      (is (= 1 (:version schema)))
      (is (= 2 (count (:required schema))))
      (is (contains? (:required schema) :user-id)))))

(deftest test-get-schema-version
  (testing "Getting schema version"
    (let [schema (i/make-schema :name :test :version 5 :fields {} :required [])]
      (is (= 5 (i/get-schema-version schema))))))

(deftest test-get-required-fields
  (testing "Getting required fields"
    (let [schema (i/make-schema
                   :name :test
                   :version 1
                   :fields {}
                   :required [:field1 :field2])]
      (is (= #{:field1 :field2} (i/get-required-fields schema))))))

;; ============================================================================
;; Migration Step Tests
;; ============================================================================

(deftest test-create-migration-step
  (testing "Creating migration step"
    (let [step (i/make-migration-step
                 :from-version 1
                 :to-version 2
                 :transform-fn (fn [e] (assoc e :migrated true)))]
      (is (= 1 (:from-version step)))
      (is (= 2 (:to-version step)))
      (is (fn? (:transform-fn step)))
      (is (= false (:reversible step))))))

(deftest test-migration-step-validation
  (testing "Valid migration step"
    (let [step (i/make-migration-step
                 :from-version 1
                 :to-version 2
                 :transform-fn identity)]
      (is (i/valid-migration-step? step)))))

(deftest test-reversible-migration-step
  (testing "Reversible migration step"
    (let [step (i/make-migration-step
                 :from-version 1
                 :to-version 2
                 :transform-fn (fn [e] (assoc e :new-field true))
                 :reversible true
                 :reverse-fn (fn [e] (dissoc e :new-field)))]
      (is (:reversible step))
      (is (fn? (:reverse-fn step))))))

;; ============================================================================
;; Migration Config Tests
;; ============================================================================

(deftest test-create-migration-config
  (testing "Creating migration configuration"
    (let [step1 (i/make-migration-step
                  :from-version 1 :to-version 2 :transform-fn identity)
          step2 (i/make-migration-step
                  :from-version 2 :to-version 3 :transform-fn identity)
          config (i/make-migration-config
                   :schema-name :user-schema
                   :steps [step1 step2])]
      (is (= :user-schema (:schema-name config)))
      (is (= 2 (count (:steps config))))
      (is (= true (:validate-after config)))
      (is (= false (:dry-run config))))))

(deftest test-migration-steps-sorted
  (testing "Migration steps are sorted by from-version"
    (let [step1 (i/make-migration-step
                  :from-version 3 :to-version 4 :transform-fn identity)
          step2 (i/make-migration-step
                  :from-version 1 :to-version 2 :transform-fn identity)
          step3 (i/make-migration-step
                  :from-version 2 :to-version 3 :transform-fn identity)
          config (i/make-migration-config
                   :schema-name :test
                   :steps [step1 step2 step3])]
      ;; Should be sorted by from-version
      (is (= [1 2 3] (map :from-version (:steps config)))))))

;; ============================================================================
;; Common Migration Steps Tests
;; ============================================================================

(deftest test-rename-field-migration
  (testing "Rename field migration"
    (let [step (i/rename-field-migration 1 2 :old-name :new-name)
          event {:old-name "value" :other "data"}
          transformed ((:transform-fn step) event)]
      (is (= "value" (:new-name transformed)))
      (is (not (contains? transformed :old-name)))
      ;; Test reverse
      (let [reversed ((:reverse-fn step) transformed)]
        (is (= "value" (:old-name reversed)))
        (is (not (contains? reversed :new-name)))))))

(deftest test-add-field-migration
  (testing "Add field migration"
    (let [step (i/add-field-migration 1 2 :new-field "default-value")
          event {:existing "value"}
          transformed ((:transform-fn step) event)]
      (is (= "default-value" (:new-field transformed)))
      (is (= "value" (:existing transformed)))
      ;; Test reverse
      (let [reversed ((:reverse-fn step) transformed)]
        (is (not (contains? reversed :new-field)))))))

(deftest test-remove-field-migration
  (testing "Remove field migration"
    (let [step (i/remove-field-migration 1 2 :deprecated-field)
          event {:deprecated-field "value" :keep-field "keep"}
          transformed ((:transform-fn step) event)]
      (is (not (contains? transformed :deprecated-field)))
      (is (= "keep" (:keep-field transformed)))
      (is (= false (:reversible step))))))

(deftest test-transform-field-migration
  (testing "Transform field migration"
    (let [step (i/transform-field-migration 1 2 :amount #(* % 100))
          event {:amount 1.5}
          transformed ((:transform-fn step) event)]
      (is (= 150.0 (:amount transformed))))))

(deftest test-migration-preserves-other-fields
  (testing "Migrations preserve other fields"
    (let [step (i/add-field-migration 1 2 :new-field "default")
          event {:field1 "a" :field2 "b" :field3 "c"}
          transformed ((:transform-fn step) event)]
      (is (= "a" (:field1 transformed)))
      (is (= "b" (:field2 transformed)))
      (is (= "c" (:field3 transformed)))
      (is (= "default" (:new-field transformed))))))
