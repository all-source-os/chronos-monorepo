defmodule QueryServiceEx.Domain.Entities.QueryTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Domain.Entities.Query
  alias QueryServiceEx.Domain.Entities.Query.{Predicate, Aggregation, SortOrder}

  doctest Query

  describe "Query.new/1" do
    test "creates a query with default values" do
      query = Query.new()

      assert query.select == [:*]
      assert query.from == :events
      assert is_nil(query.where)
      assert is_nil(query.order_by)
      assert is_nil(query.limit)
      assert is_nil(query.offset)
    end

    test "creates a query with custom values" do
      query = Query.new(
        select: [:entity_id, :event_type],
        from: :projections,
        limit: 100,
        offset: 50
      )

      assert query.select == [:entity_id, :event_type]
      assert query.from == :projections
      assert query.limit == 100
      assert query.offset == 50
    end
  end

  describe "Predicate" do
    test "creates a predicate with operator, field, and value" do
      predicate = Predicate.new(:eq, :event_type, "user.created")

      assert predicate.operator == :eq
      assert predicate.field == :event_type
      assert predicate.value == "user.created"
    end

    test "creates a logical predicate with nil field" do
      predicate = Predicate.new(:and, nil, [])

      assert predicate.operator == :and
      assert is_nil(predicate.field)
    end
  end

  describe "Aggregation" do
    test "creates an aggregation with function, field, and alias" do
      agg = Aggregation.new(:count, :*, :total_count)

      assert agg.function == :count
      assert agg.field == :*
      assert agg.alias == :total_count
    end

    test "creates sum aggregation" do
      agg = Aggregation.new(:sum, :amount, :total_amount)

      assert agg.function == :sum
      assert agg.field == :amount
      assert agg.alias == :total_amount
    end
  end

  describe "SortOrder" do
    test "creates sort order with field and direction" do
      sort = SortOrder.new(:timestamp, :desc)

      assert sort.field == :timestamp
      assert sort.direction == :desc
    end

    test "defaults to ascending direction" do
      sort = SortOrder.new(:timestamp)

      assert sort.field == :timestamp
      assert sort.direction == :asc
    end
  end

  describe "valid_operator?/1" do
    test "validates valid operators" do
      assert Query.valid_operator?(:eq)
      assert Query.valid_operator?(:ne)
      assert Query.valid_operator?(:gt)
      assert Query.valid_operator?(:gte)
      assert Query.valid_operator?(:lt)
      assert Query.valid_operator?(:lte)
      assert Query.valid_operator?(:contains)
      assert Query.valid_operator?(:in)
      assert Query.valid_operator?(:not_in)
      assert Query.valid_operator?(:between)
      assert Query.valid_operator?(:and)
      assert Query.valid_operator?(:or)
      assert Query.valid_operator?(:not)
    end

    test "rejects invalid operators" do
      refute Query.valid_operator?(:invalid)
      refute Query.valid_operator?(:foo)
      refute Query.valid_operator?(nil)
    end
  end

  describe "valid_aggregation_function?/1" do
    test "validates valid aggregation functions" do
      assert Query.valid_aggregation_function?(:count)
      assert Query.valid_aggregation_function?(:sum)
      assert Query.valid_aggregation_function?(:avg)
      assert Query.valid_aggregation_function?(:min)
      assert Query.valid_aggregation_function?(:max)
      assert Query.valid_aggregation_function?(:count_distinct)
      assert Query.valid_aggregation_function?(:percentile)
      assert Query.valid_aggregation_function?(:first)
      assert Query.valid_aggregation_function?(:last)
    end

    test "rejects invalid aggregation functions" do
      refute Query.valid_aggregation_function?(:invalid)
      refute Query.valid_aggregation_function?(:median)
    end
  end

  describe "valid_sort_direction?/1" do
    test "validates valid sort directions" do
      assert Query.valid_sort_direction?(:asc)
      assert Query.valid_sort_direction?(:desc)
    end

    test "rejects invalid sort directions" do
      refute Query.valid_sort_direction?(:invalid)
      refute Query.valid_sort_direction?(:ascending)
    end
  end

  describe "valid_predicate?/1" do
    test "validates a valid predicate" do
      predicate = Predicate.new(:eq, :event_type, "user.created")

      assert Query.valid_predicate?(predicate)
    end

    test "validates logical predicates with nil field" do
      predicate = Predicate.new(:and, nil, [])

      assert Query.valid_predicate?(predicate)
    end

    test "rejects predicate with invalid operator" do
      predicate = %Predicate{operator: :invalid, field: :event_type, value: "test"}

      refute Query.valid_predicate?(predicate)
    end

    test "rejects non-predicate structs" do
      refute Query.valid_predicate?(%{})
      refute Query.valid_predicate?(nil)
    end
  end

  describe "valid_query?/1" do
    test "validates a valid query" do
      query = Query.new(from: :events)

      assert Query.valid_query?(query)
    end

    test "validates query with predicate" do
      predicate = Predicate.new(:eq, :event_type, "user.created")
      query = Query.new(from: :events, where: predicate)

      assert Query.valid_query?(query)
    end

    test "rejects query with invalid predicate" do
      invalid_predicate = %Predicate{operator: :invalid, field: :event_type, value: "test"}
      query = %Query{from: :events, where: invalid_predicate}

      refute Query.valid_query?(query)
    end

    test "rejects non-query structs" do
      refute Query.valid_query?(%{})
      refute Query.valid_query?(nil)
    end
  end

  describe "add_predicate/2" do
    test "adds a predicate to a query" do
      query = Query.new()
      predicate = Predicate.new(:eq, :event_type, "user.created")

      updated_query = Query.add_predicate(query, predicate)

      assert updated_query.where == predicate
    end

    test "raises on invalid predicate" do
      query = Query.new()
      invalid_predicate = %Predicate{operator: :invalid, field: :event_type, value: "test"}

      assert_raise ArgumentError, fn ->
        Query.add_predicate(query, invalid_predicate)
      end
    end
  end

  describe "add_order_by/2" do
    test "adds sort orders to a query" do
      query = Query.new()
      sort_orders = [SortOrder.new(:timestamp, :desc), SortOrder.new(:entity_id, :asc)]

      updated_query = Query.add_order_by(query, sort_orders)

      assert updated_query.order_by == sort_orders
    end
  end

  describe "add_limit/2" do
    test "adds a limit to a query" do
      query = Query.new()

      updated_query = Query.add_limit(query, 100)

      assert updated_query.limit == 100
    end

    test "raises on invalid limit" do
      query = Query.new()

      assert_raise FunctionClauseError, fn ->
        Query.add_limit(query, -1)
      end

      assert_raise FunctionClauseError, fn ->
        Query.add_limit(query, 0)
      end
    end
  end

  describe "add_offset/2" do
    test "adds an offset to a query" do
      query = Query.new()

      updated_query = Query.add_offset(query, 50)

      assert updated_query.offset == 50
    end

    test "allows zero offset" do
      query = Query.new()

      updated_query = Query.add_offset(query, 0)

      assert updated_query.offset == 0
    end

    test "raises on negative offset" do
      query = Query.new()

      assert_raise FunctionClauseError, fn ->
        Query.add_offset(query, -1)
      end
    end
  end

  describe "combine_predicates/2" do
    test "combines predicates with AND" do
      pred1 = Predicate.new(:eq, :event_type, "user.created")
      pred2 = Predicate.new(:gt, :timestamp, ~U[2024-01-01 00:00:00Z])

      combined = Query.combine_predicates(:and, [pred1, pred2])

      assert combined.operator == :and
      assert is_nil(combined.field)
      assert combined.value == [pred1, pred2]
    end

    test "combines predicates with OR" do
      pred1 = Predicate.new(:eq, :event_type, "user.created")
      pred2 = Predicate.new(:eq, :event_type, "user.updated")

      combined = Query.combine_predicates(:or, [pred1, pred2])

      assert combined.operator == :or
      assert combined.value == [pred1, pred2]
    end

    test "raises on invalid operator" do
      pred1 = Predicate.new(:eq, :event_type, "user.created")

      assert_raise FunctionClauseError, fn ->
        Query.combine_predicates(:invalid, [pred1])
      end
    end
  end

  describe "select/2" do
    test "sets select fields" do
      query = Query.new()
      fields = [:entity_id, :event_type, :timestamp]

      updated_query = Query.select(query, fields)

      assert updated_query.select == fields
    end
  end

  describe "integration: building complex queries" do
    test "builds a complete query with all components" do
      query =
        Query.new()
        |> Query.select([:entity_id, :event_type, :timestamp])
        |> Query.add_predicate(Predicate.new(:eq, :event_type, "order.placed"))
        |> Query.add_order_by([SortOrder.new(:timestamp, :desc)])
        |> Query.add_limit(100)
        |> Query.add_offset(50)

      assert query.select == [:entity_id, :event_type, :timestamp]
      assert query.where.operator == :eq
      assert query.where.field == :event_type
      assert query.where.value == "order.placed"
      assert length(query.order_by) == 1
      assert query.limit == 100
      assert query.offset == 50
    end

    test "builds a query with complex predicates" do
      type_pred = Predicate.new(:eq, :event_type, "order.placed")
      time_pred = Predicate.new(:gt, :timestamp, ~U[2024-01-01 00:00:00Z])
      combined_pred = Query.combine_predicates(:and, [type_pred, time_pred])

      query =
        Query.new()
        |> Query.add_predicate(combined_pred)
        |> Query.add_limit(50)

      assert query.where.operator == :and
      assert length(query.where.value) == 2
      assert query.limit == 50
    end
  end
end
