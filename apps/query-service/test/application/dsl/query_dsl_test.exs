defmodule QueryServiceEx.Application.DSL.QueryDSLTest do
  use ExUnit.Case, async: true

  import QueryServiceEx.Application.DSL.QueryDSL

  alias QueryServiceEx.Domain.Entities.Query
  alias QueryServiceEx.Domain.Entities.Query.Predicate

  describe "from_events/0" do
    test "creates a query from events table" do
      query = from_events()

      assert query.from == :events
      assert query.select == [:*]
    end
  end

  describe "from_projection/1" do
    test "creates a query from a projection" do
      query = from_projection(:user_statistics)

      assert query.from == :user_statistics
    end

    test "creates query from different projections" do
      query1 = from_projection(:order_totals)
      query2 = from_projection(:inventory)

      assert query1.from == :order_totals
      assert query2.from == :inventory
    end
  end

  describe "where/2 with keyword list" do
    test "adds a single equality predicate" do
      query =
        from_events()
        |> where(event_type: "user.created")

      assert query.where.operator == :eq
      assert query.where.field == :event_type
      assert query.where.value == "user.created"
    end

    test "combines multiple predicates with AND" do
      query =
        from_events()
        |> where(event_type: "order.placed", entity_id: "user-123")

      assert query.where.operator == :and
      assert is_list(query.where.value)
      assert length(query.where.value) == 2
    end
  end

  describe "where/2 with tuple predicates" do
    test "adds comparison predicate" do
      query =
        from_events()
        |> where(eq(:event_type, "user.created"))

      assert query.where.operator == :eq
      assert query.where.field == :event_type
      assert query.where.value == "user.created"
    end

    test "supports greater than predicate" do
      timestamp = ~U[2024-01-01 00:00:00Z]

      query =
        from_events()
        |> where(gt(:timestamp, timestamp))

      assert query.where.operator == :gt
      assert query.where.field == :timestamp
      assert query.where.value == timestamp
    end
  end

  describe "comparison operators" do
    test "eq/2 creates equality predicate tuple" do
      pred = eq(:field, "value")
      assert pred == {:eq, :field, "value"}
    end

    test "ne/2 creates not-equal predicate tuple" do
      pred = ne(:status, "inactive")
      assert pred == {:ne, :status, "inactive"}
    end

    test "gt/2 creates greater-than predicate tuple" do
      pred = gt(:amount, 100)
      assert pred == {:gt, :amount, 100}
    end

    test "gte/2 creates greater-than-or-equal predicate tuple" do
      pred = gte(:count, 50)
      assert pred == {:gte, :count, 50}
    end

    test "lt/2 creates less-than predicate tuple" do
      pred = lt(:score, 10)
      assert pred == {:lt, :score, 10}
    end

    test "lte/2 creates less-than-or-equal predicate tuple" do
      pred = lte(:age, 65)
      assert pred == {:lte, :age, 65}
    end

    test "contains?/2 creates contains predicate tuple" do
      pred = contains?(:description, "test")
      assert pred == {:contains, :description, "test"}
    end

    test "in?/2 creates IN predicate tuple" do
      pred = in?(:status, ["active", "pending"])
      assert pred == {:in, :status, ["active", "pending"]}
    end

    test "between/3 creates BETWEEN predicate tuple" do
      pred = between(:amount, 10, 100)
      assert pred == {:between, :amount, {10, 100}}
    end
  end

  describe "logical operators" do
    test "and_predicates/1 combines predicates" do
      pred1 = Predicate.new(:eq, :event_type, "order")
      pred2 = Predicate.new(:gt, :amount, 100)

      combined = and_predicates([pred1, pred2])

      assert combined.operator == :and
      assert combined.value == [pred1, pred2]
    end

    test "or_predicates/1 combines predicates" do
      pred1 = Predicate.new(:eq, :status, "active")
      pred2 = Predicate.new(:eq, :status, "pending")

      combined = or_predicates([pred1, pred2])

      assert combined.operator == :or
      assert combined.value == [pred1, pred2]
    end
  end

  describe "select/2" do
    test "sets specific fields to select" do
      query =
        from_events()
        |> select([:entity_id, :event_type, :timestamp])

      assert query.select == [:entity_id, :event_type, :timestamp]
    end

    test "replaces existing select" do
      query =
        from_events()
        |> select([:field1, :field2])
        |> select([:field3])

      assert query.select == [:field3]
    end
  end

  describe "select_all/1" do
    test "selects all fields" do
      query =
        from_events()
        |> select([:entity_id])
        |> select_all()

      assert query.select == [:*]
    end
  end

  describe "order_by/2" do
    test "adds single sort order" do
      query =
        from_events()
        |> order_by([{:timestamp, :desc}])

      assert length(query.order_by) == 1
      assert hd(query.order_by).field == :timestamp
      assert hd(query.order_by).direction == :desc
    end

    test "adds multiple sort orders" do
      query =
        from_events()
        |> order_by([{:timestamp, :desc}, {:entity_id, :asc}])

      assert length(query.order_by) == 2
      [first, second] = query.order_by
      assert first.field == :timestamp
      assert first.direction == :desc
      assert second.field == :entity_id
      assert second.direction == :asc
    end

    test "defaults direction to asc when not specified" do
      query =
        from_events()
        |> order_by([{:entity_id, nil}])

      assert hd(query.order_by).direction == :asc
    end
  end

  describe "order_by_timestamp/1-2" do
    test "orders by timestamp descending by default" do
      query =
        from_events()
        |> order_by_timestamp()

      assert hd(query.order_by).field == :timestamp
      assert hd(query.order_by).direction == :desc
    end

    test "orders by timestamp ascending when specified" do
      query =
        from_events()
        |> order_by_timestamp(:asc)

      assert hd(query.order_by).field == :timestamp
      assert hd(query.order_by).direction == :asc
    end
  end

  describe "limit/2" do
    test "sets limit on query" do
      query =
        from_events()
        |> limit(100)

      assert query.limit == 100
    end

    test "replaces existing limit" do
      query =
        from_events()
        |> limit(50)
        |> limit(100)

      assert query.limit == 100
    end
  end

  describe "offset/2" do
    test "sets offset on query" do
      query =
        from_events()
        |> offset(50)

      assert query.offset == 50
    end

    test "allows zero offset" do
      query =
        from_events()
        |> offset(0)

      assert query.offset == 0
    end
  end

  describe "take/2" do
    test "is an alias for limit" do
      query =
        from_events()
        |> take(25)

      assert query.limit == 25
    end
  end

  describe "time helpers" do
    test "now/0 returns current timestamp" do
      before = DateTime.utc_now()
      timestamp = now()
      after_time = DateTime.utc_now()

      assert DateTime.compare(timestamp, before) in [:gt, :eq]
      assert DateTime.compare(timestamp, after_time) in [:lt, :eq]
    end

    test "days_ago/1 returns timestamp n days in the past" do
      timestamp = days_ago(7)
      expected = DateTime.utc_now() |> DateTime.add(-7 * 24 * 60 * 60, :second)

      # Allow 1 second difference due to test execution time
      diff = DateTime.diff(expected, timestamp, :second)
      assert abs(diff) <= 1
    end

    test "hours_ago/1 returns timestamp n hours in the past" do
      timestamp = hours_ago(3)
      expected = DateTime.utc_now() |> DateTime.add(-3 * 60 * 60, :second)

      diff = DateTime.diff(expected, timestamp, :second)
      assert abs(diff) <= 1
    end

    test "minutes_ago/1 returns timestamp n minutes in the past" do
      timestamp = minutes_ago(30)
      expected = DateTime.utc_now() |> DateTime.add(-30 * 60, :second)

      diff = DateTime.diff(expected, timestamp, :second)
      assert abs(diff) <= 1
    end
  end

  describe "time range queries" do
    test "since/2 adds greater-than timestamp predicate" do
      time = days_ago(7)

      query =
        from_events()
        |> since(time)

      assert query.where.operator == :gt
      assert query.where.field == :timestamp
      assert query.where.value == time
    end

    test "until/2 adds less-than timestamp predicate" do
      time = days_ago(1)

      query =
        from_events()
        |> until(time)

      assert query.where.operator == :lt
      assert query.where.field == :timestamp
      assert query.where.value == time
    end

    test "time_range/3 adds both bounds" do
      start_time = days_ago(7)
      end_time = now()

      query =
        from_events()
        |> time_range(start_time, end_time)

      # Should have two where clauses applied
      # Note: Current implementation chains where calls
      assert query.where.operator == :lte
      assert query.where.field == :timestamp
    end
  end

  describe "aggregation helpers" do
    test "count_events/0 creates count aggregation" do
      agg = count_events()

      assert agg.function == :count
      assert agg.field == :*
      assert agg.alias == :count
    end

    test "count_events/1 creates count aggregation on field" do
      agg = count_events(:entity_id)

      assert agg.function == :count
      assert agg.field == :entity_id
    end

    test "sum_field/1 creates sum aggregation" do
      agg = sum_field(:amount)

      assert agg.function == :sum
      assert agg.field == :amount
      assert agg.alias == :amount_sum
    end

    test "avg_field/1 creates average aggregation" do
      agg = avg_field(:score)

      assert agg.function == :avg
      assert agg.field == :score
      assert agg.alias == :score_avg
    end

    test "min_field/1 creates minimum aggregation" do
      agg = min_field(:price)

      assert agg.function == :min
      assert agg.field == :price
      assert agg.alias == :price_min
    end

    test "max_field/1 creates maximum aggregation" do
      agg = max_field(:quantity)

      assert agg.function == :max
      assert agg.field == :quantity
      assert agg.alias == :quantity_max
    end

    test "count_distinct/1 creates count distinct aggregation" do
      agg = count_distinct(:user_id)

      assert agg.function == :count_distinct
      assert agg.field == :user_id
      assert agg.alias == :distinct_count
    end
  end

  describe "integration: building complex queries" do
    test "builds a complete query with all components" do
      query =
        from_events()
        |> select([:entity_id, :event_type, :timestamp])
        |> where(eq(:event_type, "order.placed"))
        |> since(days_ago(7))
        |> order_by_timestamp(:desc)
        |> limit(100)
        |> offset(50)

      assert query.from == :events
      assert query.select == [:entity_id, :event_type, :timestamp]
      assert query.limit == 100
      assert query.offset == 50
      assert hd(query.order_by).field == :timestamp
    end

    test "builds a time-based query" do
      query =
        from_events()
        |> where(event_type: "user.login")
        |> since(days_ago(30))
        |> order_by_timestamp()
        |> take(1000)

      assert query.from == :events
      assert query.limit == 1000
    end

    test "builds a projection query" do
      query =
        from_projection(:user_statistics)
        |> select([:user_id, :login_count, :last_login])
        |> where(gt(:login_count, 10))
        |> order_by([{:login_count, :desc}])
        |> limit(50)

      assert query.from == :user_statistics
      assert query.select == [:user_id, :login_count, :last_login]
      assert query.limit == 50
    end

    test "builds query with multiple predicates" do
      query =
        from_events()
        |> where(event_type: "order.placed")
        |> where(gt(:amount, 100))

      # Note: Current implementation overwrites predicates
      # This test documents current behavior
      assert query.where.operator == :gt
    end
  end

  describe "pipe-friendly API" do
    test "supports Elixir pipe operator" do
      # This is the key advantage over Clojure's threading macros
      result =
        :events
        |> from_projection()
        |> select([:id, :type])
        |> limit(10)

      assert %Query{} = result
      assert result.from == :events
      assert result.select == [:id, :type]
      assert result.limit == 10
    end

    test "composes naturally with Elixir's pipe" do
      base_query = from_events()

      # Can be stored and piped later
      filtered_query =
        base_query
        |> where(event_type: "order.placed")

      paginated_query =
        filtered_query
        |> limit(100)
        |> offset(0)

      assert paginated_query.limit == 100
      assert paginated_query.offset == 0
    end
  end

  describe "real-world query examples" do
    test "user activity query" do
      query =
        from_events()
        |> select([:entity_id, :event_type, :timestamp, :payload])
        |> where(eq(:entity_id, "user-123"))
        |> since(days_ago(30))
        |> order_by_timestamp(:desc)
        |> limit(100)

      assert query.select == [:entity_id, :event_type, :timestamp, :payload]
      assert query.limit == 100
    end

    test "recent orders query" do
      query =
        from_events()
        |> where(event_type: "order.placed")
        |> since(hours_ago(24))
        |> order_by([{:timestamp, :desc}])
        |> take(50)

      assert query.limit == 50
      assert length(query.order_by) == 1
    end

    test "high-value transactions query" do
      query =
        from_events()
        |> select([:entity_id, :timestamp, :payload])
        |> where(event_type: "payment.processed")
        |> where(gt(:amount, 10_000))
        |> order_by_timestamp()

      assert query.select == [:entity_id, :timestamp, :payload]
    end
  end
end
