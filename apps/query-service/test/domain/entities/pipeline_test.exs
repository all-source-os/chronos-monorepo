defmodule QueryServiceEx.Domain.Entities.PipelineTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Domain.Entities.Pipeline
  alias QueryServiceEx.Domain.Entities.Pipeline.Definition

  doctest Pipeline

  describe "Pipeline.Definition.new/1" do
    test "creates a new pipeline definition" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: []
        )

      assert pipeline.name == :test_pipeline
      assert pipeline.version == 1
      assert pipeline.operators == []
      assert pipeline.config == %{}
      assert pipeline.metadata == %{}
    end

    test "accepts optional config and metadata" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [],
          config: %{batch_size: 100},
          metadata: %{author: "test"}
        )

      assert pipeline.config == %{batch_size: 100}
      assert pipeline.metadata == %{author: "test"}
    end

    test "raises on missing required fields" do
      assert_raise ArgumentError, fn ->
        Definition.new(name: :test_pipeline)
      end

      assert_raise ArgumentError, fn ->
        Definition.new(version: 1, operators: [])
      end
    end
  end

  describe "Pipeline.valid_version?/1" do
    test "accepts positive integers" do
      assert Pipeline.valid_version?(1)
      assert Pipeline.valid_version?(100)
      assert Pipeline.valid_version?(999)
    end

    test "rejects zero and negative numbers" do
      refute Pipeline.valid_version?(0)
      refute Pipeline.valid_version?(-1)
      refute Pipeline.valid_version?(-100)
    end

    test "rejects non-integers" do
      refute Pipeline.valid_version?(1.5)
      refute Pipeline.valid_version?("1")
      refute Pipeline.valid_version?(nil)
      refute Pipeline.valid_version?(:one)
    end
  end

  describe "Pipeline.valid_name?/1" do
    test "accepts atoms" do
      assert Pipeline.valid_name?(:test_pipeline)
      assert Pipeline.valid_name?(:user_events)
      assert Pipeline.valid_name?(:order_processor)
    end

    test "rejects non-atoms" do
      refute Pipeline.valid_name?("test_pipeline")
      refute Pipeline.valid_name?(123)
      refute Pipeline.valid_name?(%{})
      refute Pipeline.valid_name?([])
    end
  end

  describe "Pipeline.valid_operator_type?/1" do
    test "accepts all valid operator types" do
      assert Pipeline.valid_operator_type?(:filter)
      assert Pipeline.valid_operator_type?(:transform)
      assert Pipeline.valid_operator_type?(:enrich)
      assert Pipeline.valid_operator_type?(:aggregate)
      assert Pipeline.valid_operator_type?(:route)
      assert Pipeline.valid_operator_type?(:validate)
    end

    test "rejects invalid operator types" do
      refute Pipeline.valid_operator_type?(:map)
      refute Pipeline.valid_operator_type?(:reduce)
      refute Pipeline.valid_operator_type?(:unknown)
      refute Pipeline.valid_operator_type?("filter")
      refute Pipeline.valid_operator_type?(nil)
    end
  end

  describe "Pipeline.valid_operator?/1" do
    test "accepts valid operator maps" do
      operator = %{type: :filter, name: "test_filter"}
      assert Pipeline.valid_operator?(operator)

      operator = %{type: :transform, name: "add_timestamp"}
      assert Pipeline.valid_operator?(operator)
    end

    test "rejects operators with invalid type" do
      operator = %{type: :invalid, name: "test"}
      refute Pipeline.valid_operator?(operator)
    end

    test "rejects operators with non-string name" do
      operator = %{type: :filter, name: :test}
      refute Pipeline.valid_operator?(operator)

      operator = %{type: :filter, name: 123}
      refute Pipeline.valid_operator?(operator)
    end

    test "rejects operators missing required fields" do
      refute Pipeline.valid_operator?(%{type: :filter})
      refute Pipeline.valid_operator?(%{name: "test"})
      refute Pipeline.valid_operator?(%{})
    end
  end

  describe "Pipeline.valid_pipeline?/1" do
    test "accepts valid pipeline definition" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            %{type: :filter, name: "filter_orders"},
            %{type: :transform, name: "add_timestamp"}
          ]
        )

      assert Pipeline.valid_pipeline?(pipeline)
    end

    test "accepts pipeline with empty operators list" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: []
        )

      assert Pipeline.valid_pipeline?(pipeline)
    end

    test "rejects pipeline with invalid name" do
      pipeline = %Definition{
        name: "invalid_name",
        version: 1,
        operators: []
      }

      refute Pipeline.valid_pipeline?(pipeline)
    end

    test "rejects pipeline with invalid version" do
      pipeline = %Definition{
        name: :test_pipeline,
        version: 0,
        operators: []
      }

      refute Pipeline.valid_pipeline?(pipeline)
    end

    test "rejects pipeline with invalid operators" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            %{type: :invalid, name: "bad_op"}
          ]
        )

      refute Pipeline.valid_pipeline?(pipeline)
    end

    test "rejects non-Definition structs" do
      refute Pipeline.valid_pipeline?(%{})
      refute Pipeline.valid_pipeline?(nil)
      refute Pipeline.valid_pipeline?("pipeline")
    end
  end

  describe "Pipeline.filter_operator/2" do
    test "creates a filter operator with predicate function" do
      operator =
        Pipeline.filter_operator("only_orders", fn event ->
          event.event_type == "order.placed"
        end)

      assert operator.type == :filter
      assert operator.name == "only_orders"
      assert operator.config == %{}
      assert is_function(operator.function, 1)
    end

    test "filter function is callable" do
      operator =
        Pipeline.filter_operator("test", fn event ->
          event.value > 100
        end)

      assert operator.function.(%{value: 150})
      refute operator.function.(%{value: 50})
    end
  end

  describe "Pipeline.transform_operator/2" do
    test "creates a transform operator with transform function" do
      operator =
        Pipeline.transform_operator("add_timestamp", fn event ->
          Map.put(event, :processed_at, DateTime.utc_now())
        end)

      assert operator.type == :transform
      assert operator.name == "add_timestamp"
      assert operator.config == %{}
      assert is_function(operator.function, 1)
    end

    test "transform function modifies event" do
      operator =
        Pipeline.transform_operator("uppercase", fn event ->
          Map.update!(event, :name, &String.upcase/1)
        end)

      result = operator.function.(%{name: "test"})
      assert result.name == "TEST"
    end
  end

  describe "Pipeline.enrich_operator/2" do
    test "creates an enrich operator with enrich function" do
      operator =
        Pipeline.enrich_operator("add_user_data", fn event ->
          Map.put(event, :user, %{id: event.user_id, name: "Test User"})
        end)

      assert operator.type == :enrich
      assert operator.name == "add_user_data"
      assert is_function(operator.function, 1)
    end

    test "enrich function adds data to event" do
      operator =
        Pipeline.enrich_operator("add_metadata", fn event ->
          Map.put(event, :metadata, %{source: "api", version: 1})
        end)

      result = operator.function.(%{id: 1})
      assert result.metadata == %{source: "api", version: 1}
    end
  end

  describe "Pipeline.validate_operator/2" do
    test "creates a validate operator with validation function" do
      operator =
        Pipeline.validate_operator("validate_schema", fn event ->
          if Map.has_key?(event, :required_field), do: event, else: raise("Missing field")
        end)

      assert operator.type == :validate
      assert operator.name == "validate_schema"
      assert is_function(operator.function, 1)
    end
  end

  describe "Pipeline.route_operator/2" do
    test "creates a route operator with routing function" do
      operator =
        Pipeline.route_operator("route_by_type", fn event ->
          case event.event_type do
            "order." <> _ -> :orders
            "user." <> _ -> :users
            _ -> :default
          end
        end)

      assert operator.type == :route
      assert operator.name == "route_by_type"
      assert is_function(operator.function, 1)
    end
  end

  describe "Pipeline.apply_operator/2 - filter" do
    test "passes event when predicate returns true" do
      operator = Pipeline.filter_operator("test", fn event -> event.active end)
      event = %{active: true, id: 1}

      assert {:ok, ^event} = Pipeline.apply_operator(event, operator)
    end

    test "filters event when predicate returns false" do
      operator = Pipeline.filter_operator("test", fn event -> event.active end)
      event = %{active: false, id: 1}

      assert :filtered == Pipeline.apply_operator(event, operator)
    end
  end

  describe "Pipeline.apply_operator/2 - transform" do
    test "transforms event successfully" do
      operator =
        Pipeline.transform_operator("add_field", fn event ->
          Map.put(event, :processed, true)
        end)

      event = %{id: 1}
      assert {:ok, result} = Pipeline.apply_operator(event, operator)
      assert result.processed == true
    end

    test "returns error when transform function raises" do
      operator =
        Pipeline.transform_operator("fail", fn _event ->
          raise "Transform error"
        end)

      event = %{id: 1}
      assert {:error, "Transform error"} = Pipeline.apply_operator(event, operator)
    end
  end

  describe "Pipeline.apply_operator/2 - enrich" do
    test "enriches event successfully" do
      operator =
        Pipeline.enrich_operator("add_data", fn event ->
          Map.put(event, :extra, "data")
        end)

      event = %{id: 1}
      assert {:ok, result} = Pipeline.apply_operator(event, operator)
      assert result.extra == "data"
    end

    test "returns error when enrich function raises" do
      operator =
        Pipeline.enrich_operator("fail", fn _event ->
          raise RuntimeError, "Enrich error"
        end)

      event = %{id: 1}
      assert {:error, "Enrich error"} = Pipeline.apply_operator(event, operator)
    end
  end

  describe "Pipeline.apply_operator/2 - validate" do
    test "validates event successfully" do
      operator =
        Pipeline.validate_operator("check", fn event ->
          if event.valid, do: event, else: raise("Invalid")
        end)

      event = %{valid: true}
      assert {:ok, ^event} = Pipeline.apply_operator(event, operator)
    end

    test "returns error when validation fails" do
      operator =
        Pipeline.validate_operator("check", fn _event ->
          raise "Validation failed"
        end)

      event = %{valid: false}
      assert {:error, "Validation failed"} = Pipeline.apply_operator(event, operator)
    end
  end

  describe "Pipeline.apply_operator/2 - route" do
    test "routes event and adds route metadata" do
      operator =
        Pipeline.route_operator("router", fn event ->
          if event.priority == :high, do: :priority_queue, else: :normal_queue
        end)

      event = %{priority: :high, id: 1}
      assert {:ok, result} = Pipeline.apply_operator(event, operator)
      assert result.__route__ == :priority_queue
    end

    test "returns error when routing function raises" do
      operator =
        Pipeline.route_operator("router", fn _event ->
          raise "Routing error"
        end)

      event = %{id: 1}
      assert {:error, "Routing error"} = Pipeline.apply_operator(event, operator)
    end
  end

  describe "Pipeline.apply_pipeline/2" do
    test "applies all operators in sequence" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.filter_operator("active_only", fn e -> e.active end),
            Pipeline.transform_operator("add_timestamp", fn e ->
              Map.put(e, :timestamp, "2024-01-01")
            end),
            Pipeline.enrich_operator("add_source", fn e ->
              Map.put(e, :source, "pipeline")
            end)
          ]
        )

      event = %{id: 1, active: true}
      assert {:ok, result} = Pipeline.apply_pipeline(event, pipeline)
      assert result.id == 1
      assert result.active == true
      assert result.timestamp == "2024-01-01"
      assert result.source == "pipeline"
    end

    test "stops at first filter that rejects" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.filter_operator("first", fn e -> e.active end),
            Pipeline.filter_operator("second", fn e -> e.premium end),
            Pipeline.transform_operator("should_not_run", fn e ->
              Map.put(e, :processed, true)
            end)
          ]
        )

      event = %{id: 1, active: true, premium: false}
      assert :filtered == Pipeline.apply_pipeline(event, pipeline)
    end

    test "stops at first error" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.transform_operator("first", fn e -> Map.put(e, :step1, true) end),
            Pipeline.transform_operator("error", fn _e -> raise "Error!" end),
            Pipeline.transform_operator("should_not_run", fn e ->
              Map.put(e, :step3, true)
            end)
          ]
        )

      event = %{id: 1}
      assert {:error, "Error!"} = Pipeline.apply_pipeline(event, pipeline)
    end

    test "handles empty operators list" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: []
        )

      event = %{id: 1}
      assert {:ok, ^event} = Pipeline.apply_pipeline(event, pipeline)
    end
  end

  describe "Pipeline.apply_pipeline_batch/2" do
    test "applies pipeline to multiple events" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.transform_operator("add_processed", fn e ->
              Map.put(e, :processed, true)
            end)
          ]
        )

      events = [%{id: 1}, %{id: 2}, %{id: 3}]
      results = Pipeline.apply_pipeline_batch(events, pipeline)

      assert length(results) == 3
      assert Enum.all?(results, fn e -> e.processed == true end)
    end

    test "filters out rejected events" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.filter_operator("even_only", fn e -> rem(e.id, 2) == 0 end)
          ]
        )

      events = [%{id: 1}, %{id: 2}, %{id: 3}, %{id: 4}]
      results = Pipeline.apply_pipeline_batch(events, pipeline)

      assert length(results) == 2
      assert Enum.map(results, & &1.id) == [2, 4]
    end

    test "filters out events that errored" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [
            Pipeline.transform_operator("fail_odd", fn e ->
              if rem(e.id, 2) == 0, do: e, else: raise("Odd number")
            end)
          ]
        )

      events = [%{id: 1}, %{id: 2}, %{id: 3}, %{id: 4}]
      results = Pipeline.apply_pipeline_batch(events, pipeline)

      assert length(results) == 2
      assert Enum.map(results, & &1.id) == [2, 4]
    end

    test "handles empty event list" do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 1,
          operators: [Pipeline.filter_operator("test", fn _ -> true end)]
        )

      assert [] == Pipeline.apply_pipeline_batch([], pipeline)
    end
  end

  describe "Pipeline accessors" do
    setup do
      pipeline =
        Definition.new(
          name: :test_pipeline,
          version: 2,
          operators: [%{type: :filter, name: "test"}],
          config: %{batch_size: 100},
          metadata: %{author: "test"}
        )

      {:ok, pipeline: pipeline}
    end

    test "get_name/1 returns pipeline name", %{pipeline: pipeline} do
      assert Pipeline.get_name(pipeline) == :test_pipeline
    end

    test "get_version/1 returns pipeline version", %{pipeline: pipeline} do
      assert Pipeline.get_version(pipeline) == 2
    end

    test "get_operators/1 returns operators list", %{pipeline: pipeline} do
      operators = Pipeline.get_operators(pipeline)
      assert length(operators) == 1
      assert hd(operators).type == :filter
    end

    test "get_config/1 returns config map", %{pipeline: pipeline} do
      assert Pipeline.get_config(pipeline) == %{batch_size: 100}
    end

    test "get_metadata/1 returns metadata map", %{pipeline: pipeline} do
      assert Pipeline.get_metadata(pipeline) == %{author: "test"}
    end

    test "operator_count/1 returns number of operators", %{pipeline: pipeline} do
      assert Pipeline.operator_count(pipeline) == 1
    end
  end

  describe "real-world pipeline scenarios" do
    test "order processing pipeline" do
      pipeline =
        Definition.new(
          name: :order_pipeline,
          version: 1,
          operators: [
            # Only process order events
            Pipeline.filter_operator("orders_only", fn e ->
              String.starts_with?(e.event_type, "order.")
            end),
            # Validate required fields
            Pipeline.validate_operator("validate", fn e ->
              if Map.has_key?(e, :order_id) and Map.has_key?(e, :amount),
                do: e,
                else: raise("Missing required fields")
            end),
            # Enrich with customer data
            Pipeline.enrich_operator("add_customer", fn e ->
              Map.put(e, :customer, %{id: e.customer_id, tier: "premium"})
            end),
            # Transform amount to cents
            Pipeline.transform_operator("to_cents", fn e ->
              Map.update!(e, :amount, &(&1 * 100))
            end),
            # Route based on amount
            Pipeline.route_operator("route_by_amount", fn e ->
              if e.amount > 10_000, do: :high_value, else: :standard
            end)
          ]
        )

      event = %{
        event_type: "order.placed",
        order_id: "ORD-123",
        customer_id: "CUST-456",
        amount: 150.50
      }

      assert {:ok, result} = Pipeline.apply_pipeline(event, pipeline)
      assert result.amount == 15_050
      assert result.customer.tier == "premium"
      assert result.__route__ == :high_value
    end

    test "user activity pipeline with filtering" do
      pipeline =
        Definition.new(
          name: :activity_pipeline,
          version: 1,
          operators: [
            # Filter out bot traffic
            Pipeline.filter_operator("no_bots", fn e ->
              not Map.get(e, :is_bot, false)
            end),
            # Filter active users only
            Pipeline.filter_operator("active_users", fn e ->
              Map.get(e, :last_active_days, 999) <= 30
            end),
            # Add activity score
            Pipeline.transform_operator("score", fn e ->
              score = calculate_score(e)
              Map.put(e, :activity_score, score)
            end)
          ]
        )

      events = [
        %{user_id: 1, is_bot: false, last_active_days: 5},
        %{user_id: 2, is_bot: true, last_active_days: 1},
        %{user_id: 3, is_bot: false, last_active_days: 45},
        %{user_id: 4, is_bot: false, last_active_days: 10}
      ]

      results = Pipeline.apply_pipeline_batch(events, pipeline)
      assert length(results) == 2
      assert Enum.map(results, & &1.user_id) == [1, 4]
      assert Enum.all?(results, &Map.has_key?(&1, :activity_score))
    end

    defp calculate_score(event) do
      base = 100
      base - Map.get(event, :last_active_days, 0)
    end
  end
end
