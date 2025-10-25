(ns allsource.domain.entities.projection-test
  "TDD tests for Projection domain entities.
   RED phase - these tests will fail until we implement the code."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.projection :as p]))

(deftest test-create-projection-definition
  (testing "Creating a projection definition"
    (let [proj (p/make-projection
                 :name :user-statistics
                 :version 1
                 :initial-state {:count 0}
                 :project-fn (fn [state event] (update state :count inc)))]
      (is (= :user-statistics (:name proj)))
      (is (= 1 (:version proj)))
      (is (= {:count 0} (:initial-state proj)))
      (is (fn? (:project-fn proj))))))

(deftest test-apply-event-to-projection
  (testing "Applying an event to projection state"
    (let [project-fn (fn [state event]
                       (if (= "user.created" (:event-type event))
                         (update state :user-count inc)
                         state))
          state {:user-count 0}
          event {:event-type "user.created" :entity-id "user-1"}]
      (is (= {:user-count 1} (project-fn state event))))))

(deftest test-projection-state-immutability
  (testing "Projection state remains immutable"
    (let [project-fn (fn [state event]
                       (update state :count inc))
          original-state {:count 0}
          event {:event-type "test"}
          new-state (project-fn original-state event)]
      (is (= {:count 0} original-state))  ; Original unchanged
      (is (= {:count 1} new-state)))))

(deftest test-projection-versioning
  (testing "Projection version validation"
    (is (p/valid-version? 1))
    (is (p/valid-version? 2))
    (is (not (p/valid-version? 0)))
    (is (not (p/valid-version? -1)))
    (is (not (p/valid-version? nil)))))

(deftest test-projection-name-validation
  (testing "Projection name must be a keyword"
    (is (p/valid-name? :user-stats))
    (is (p/valid-name? :order-aggregates))
    (is (not (p/valid-name? "string-name")))
    (is (not (p/valid-name? nil)))))

(deftest test-projection-state-snapshot
  (testing "Creating projection state snapshots"
    (let [state {:count 100 :total 50000}
          snapshot (p/make-snapshot
                     :projection-name :user-stats
                     :entity-id "user-123"
                     :state state
                     :version 2
                     :timestamp (java.time.Instant/now))]
      (is (= :user-stats (:projection-name snapshot)))
      (is (= "user-123" (:entity-id snapshot)))
      (is (= state (:state snapshot)))
      (is (= 2 (:version snapshot)))
      (is (inst? (:timestamp snapshot))))))

(deftest test-projection-migration
  (testing "Migrating projection state between versions"
    (let [old-state {:count 10}
          migration-fn (fn [state]
                        (assoc state :total (* (:count state) 100)))
          new-state (p/migrate-state old-state migration-fn)]
      (is (= {:count 10 :total 1000} new-state)))))

(deftest test-projection-validation
  (testing "Valid projection definition"
    (let [valid-proj (p/make-projection
                       :name :test
                       :version 1
                       :initial-state {}
                       :project-fn (fn [s e] s))]
      (is (p/valid-projection? valid-proj))))

  (testing "Invalid projection - missing name"
    (let [invalid-proj {:version 1 :initial-state {} :project-fn (fn [s e] s)}]
      (is (not (p/valid-projection? invalid-proj)))))

  (testing "Invalid projection - invalid version"
    (let [invalid-proj (p/make-projection
                         :name :test
                         :version -1
                         :initial-state {}
                         :project-fn (fn [s e] s))]
      (is (not (p/valid-projection? invalid-proj))))))

(deftest test-get-projection-version
  (testing "Extracting version from projection"
    (let [proj (p/make-projection
                 :name :test
                 :version 3
                 :initial-state {}
                 :project-fn (fn [s e] s))]
      (is (= 3 (p/get-version proj))))))

(deftest test-get-projection-name
  (testing "Extracting name from projection"
    (let [proj (p/make-projection
                 :name :user-statistics
                 :version 1
                 :initial-state {}
                 :project-fn (fn [s e] s))]
      (is (= :user-statistics (p/get-name proj))))))
