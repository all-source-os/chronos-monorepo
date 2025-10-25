(ns allsource.domain.entities.query-test
  "Tests for domain query entities.
   Pure domain tests with no external dependencies."
  (:require [clojure.test :refer [deftest is testing]]
            [allsource.domain.entities.query :as q]))

(deftest test-make-query
  (testing "Creating a basic query"
    (let [query (q/make-query :select [:*] :from :events :limit 100)]
      (is (= [:*] (:select query)))
      (is (= :events (:from query)))
      (is (= 100 (:limit query)))
      (is (nil? (:where query))))))

(deftest test-make-predicate
  (testing "Creating predicates"
    (let [pred (q/make-predicate :eq :event-type "user.created")]
      (is (= :eq (:operator pred)))
      (is (= :event-type (:field pred)))
      (is (= "user.created" (:value pred))))))

(deftest test-valid-operator?
  (testing "Operator validation"
    (is (q/valid-operator? :eq))
    (is (q/valid-operator? :gt))
    (is (q/valid-operator? :and))
    (is (not (q/valid-operator? :invalid)))))

(deftest test-valid-predicate?
  (testing "Predicate validation"
    (let [valid-pred (q/make-predicate :eq :field "value")
          invalid-pred (q/map->Predicate {:operator :invalid :field :x})]
      (is (q/valid-predicate? valid-pred))
      (is (not (q/valid-predicate? invalid-pred))))))

(deftest test-combine-predicates
  (testing "Combining predicates with AND"
    (let [pred1 (q/make-predicate :eq :a 1)
          pred2 (q/make-predicate :eq :b 2)
          combined (q/combine-predicates :and pred1 pred2)]
      (is (= :and (:operator combined)))
      (is (= [pred1 pred2] (:value combined)))))

  (testing "Combining predicates with OR"
    (let [pred1 (q/make-predicate :eq :a 1)
          pred2 (q/make-predicate :eq :b 2)
          combined (q/combine-predicates :or pred1 pred2)]
      (is (= :or (:operator combined)))
      (is (= [pred1 pred2] (:value combined))))))

(deftest test-add-predicate
  (testing "Adding WHERE clause to query"
    (let [query (q/make-query)
          pred (q/make-predicate :eq :event-type "test")
          updated (q/add-predicate query pred)]
      (is (= pred (:where updated))))))

(deftest test-add-limit
  (testing "Adding LIMIT clause"
    (let [query (q/make-query)
          updated (q/add-limit query 50)]
      (is (= 50 (:limit updated))))))

(deftest test-add-offset
  (testing "Adding OFFSET clause"
    (let [query (q/make-query)
          updated (q/add-offset query 100)]
      (is (= 100 (:offset updated))))))

(deftest test-aggregation-functions
  (testing "Valid aggregation functions"
    (is (q/valid-aggregation-function? :count))
    (is (q/valid-aggregation-function? :sum))
    (is (q/valid-aggregation-function? :avg))
    (is (not (q/valid-aggregation-function? :invalid)))))

(deftest test-make-aggregation
  (testing "Creating aggregation"
    (let [agg (q/make-aggregation :count :* :event-count)]
      (is (= :count (:function agg)))
      (is (= :* (:field agg)))
      (is (= :event-count (:alias agg))))))
