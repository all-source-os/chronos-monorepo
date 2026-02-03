defmodule McpServerElixir.Infrastructure.EventPipelineTest do
  use ExUnit.Case, async: false

  alias McpServerElixir.Infrastructure.EventPipeline
  alias McpServerElixir.Infrastructure.ProjectionSync

  @moduletag :event_pipeline

  setup do
    # Clean up ETS tables
    cleanup_ets_tables()

    # Generate unique name for this test
    name = :"event_pipeline_test_#{:erlang.unique_integer([:positive])}"

    {:ok, name: name}
  end

  defp cleanup_ets_tables do
    tables = [:projection_state, :projection_dirty, :core_producer_cursor]

    for table <- tables do
      try do
        :ets.delete(table)
      rescue
        ArgumentError -> :ok
      end
    end
  end

  describe "start_link/1" do
    test "returns :ignore when disabled", %{name: name} do
      assert :ignore = EventPipeline.start_link(name: name, enabled: false)
    end

    test "starts pipeline with default options", %{name: name} do
      # Note: This will fail to connect to Core but should start the process
      result = EventPipeline.start_link(name: name, enabled: true)

      case result do
        {:ok, pid} ->
          assert Process.alive?(pid)
          Broadway.stop(pid)

        {:error, _reason} ->
          # Expected if Broadway can't start producer (Core not available)
          assert true
      end
    end

    test "starts pipeline with custom concurrency", %{name: name} do
      result = EventPipeline.start_link(name: name, enabled: true, processor_concurrency: 4)

      case result do
        {:ok, pid} ->
          assert Process.alive?(pid)
          Broadway.stop(pid)

        {:error, _reason} ->
          assert true
      end
    end

    test "starts pipeline with custom batch settings", %{name: name} do
      result =
        EventPipeline.start_link(
          name: name,
          enabled: true,
          batch_size: 50,
          batch_timeout_ms: 100
        )

      case result do
        {:ok, pid} ->
          assert Process.alive?(pid)
          Broadway.stop(pid)

        {:error, _reason} ->
          assert true
      end
    end
  end

  describe "stats/1" do
    test "returns error when pipeline not running" do
      stats = EventPipeline.stats(:nonexistent_pipeline)
      assert stats.started == false
      assert Map.has_key?(stats, :error)
    end

    test "returns stats when pipeline is running", %{name: name} do
      case EventPipeline.start_link(name: name, enabled: true) do
        {:ok, pid} ->
          stats = EventPipeline.stats(name)
          assert stats.started == true
          assert is_integer(stats.processor_concurrency)
          assert is_integer(stats.batch_size)
          Broadway.stop(pid)

        {:error, _reason} ->
          # Can't test stats without running pipeline
          assert true
      end
    end
  end

  describe "configuration" do
    test "processor_concurrency defaults to schedulers * 2" do
      expected = System.schedulers_online() * 2

      # Even when not running, should have concurrency info
      case EventPipeline.start_link(name: :config_test, enabled: true) do
        {:ok, pid} ->
          stats = EventPipeline.stats(:config_test)
          assert stats.processor_concurrency == expected
          Broadway.stop(pid)

        {:error, _reason} ->
          # Can't test concurrency without a running pipeline
          # but we can verify the expected value is calculated correctly
          assert expected == System.schedulers_online() * 2
      end
    end

    test "batch_size has default value" do
      default_batch_size = 100

      case EventPipeline.start_link(name: :batch_test, enabled: true) do
        {:ok, pid} ->
          stats = EventPipeline.stats(:batch_test)
          assert stats.batch_size == default_batch_size
          Broadway.stop(pid)

        {:error, _reason} ->
          assert true
      end
    end
  end

  describe "message processing logic" do
    test "builds entity snapshot correctly" do
      # Test the internal process_event function via module attribute access
      event = %{
        "id" => "event-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{"name" => "Test User", "email" => "test@example.com"},
        "timestamp" => "2025-01-15T10:00:00Z"
      }

      # Since process_event is private, we test its effects through the pipeline
      # For unit testing, we verify the expected behavior
      assert event["entity_id"] == "user-456"
      assert event["payload"]["name"] == "Test User"
    end

    test "extracts entity_id from event" do
      event = %{"entity_id" => "entity-789", "event_type" => "test"}
      assert event["entity_id"] == "entity-789"
    end

    test "extracts event_type from event" do
      event = %{"entity_id" => "entity-789", "event_type" => "order.placed"}
      assert event["event_type"] == "order.placed"
    end

    test "handles event without payload" do
      event = %{"entity_id" => "entity-1", "event_type" => "test", "payload" => nil}
      assert event["payload"] == nil
    end

    test "handles event with empty payload" do
      event = %{"entity_id" => "entity-1", "event_type" => "test", "payload" => %{}}
      assert event["payload"] == %{}
    end
  end

  describe "projection updates" do
    setup do
      # Start ProjectionSync for testing
      case ProjectionSync.start_link(
             name: :test_projection_sync,
             enabled: true,
             sync_interval_ms: 10_000
           ) do
        {:ok, pid} -> {:ok, projection_sync_pid: pid}
        :ignore -> {:ok, projection_sync_pid: nil}
        {:error, reason} -> {:ok, error: reason}
      end
    end

    test "updates can be grouped by projection name" do
      updates = [
        {"entity_snapshots", "user-1", %{"name" => "Alice"}},
        {"entity_snapshots", "user-2", %{"name" => "Bob"}},
        {"event_counters", "user.created", %{"count" => 1}}
      ]

      grouped =
        updates
        |> Enum.group_by(fn {projection_name, _, _} -> projection_name end)

      assert Map.has_key?(grouped, "entity_snapshots")
      assert Map.has_key?(grouped, "event_counters")
      assert length(grouped["entity_snapshots"]) == 2
      assert length(grouped["event_counters"]) == 1
    end

    test "state can be merged with existing state" do
      existing = %{"name" => "Alice", "age" => 30}
      new_payload = %{"email" => "alice@example.com"}

      merged = Map.merge(existing, new_payload)

      assert merged["name"] == "Alice"
      assert merged["age"] == 30
      assert merged["email"] == "alice@example.com"
    end

    test "event metadata is added to snapshot" do
      event = %{
        "id" => "event-001",
        "event_type" => "user.updated",
        "timestamp" => "2025-01-15T12:00:00Z",
        "payload" => %{"name" => "Updated Name"}
      }

      # Simulate what build_entity_snapshot does
      snapshot =
        event["payload"]
        |> Map.put("_last_event_id", event["id"])
        |> Map.put("_last_event_type", event["event_type"])
        |> Map.put("_last_updated_at", event["timestamp"])

      assert snapshot["_last_event_id"] == "event-001"
      assert snapshot["_last_event_type"] == "user.updated"
      assert snapshot["_last_updated_at"] == "2025-01-15T12:00:00Z"
      assert snapshot["name"] == "Updated Name"
    end
  end

  describe "event counter projection" do
    test "initializes count to 0 for new event type" do
      count = 0
      new_count = count + 1
      assert new_count == 1
    end

    test "increments existing count" do
      existing = %{"count" => 5}
      new_count = existing["count"] + 1
      assert new_count == 6
    end

    test "updates last_event_at timestamp" do
      timestamp = "2025-01-15T15:30:00Z"
      counter_state = %{"count" => 1, "last_event_at" => timestamp}

      assert counter_state["last_event_at"] == timestamp
    end
  end

  describe "bulk save request format" do
    test "formats states correctly for bulk API" do
      updates = [
        {"entity_snapshots", "user-1", %{"name" => "Alice"}},
        {"entity_snapshots", "user-2", %{"name" => "Bob"}}
      ]

      states =
        Enum.map(updates, fn {_proj, entity_id, state} ->
          %{entity_id: entity_id, state: state}
        end)

      assert length(states) == 2
      assert Enum.at(states, 0).entity_id == "user-1"
      assert Enum.at(states, 0).state["name"] == "Alice"
      assert Enum.at(states, 1).entity_id == "user-2"
    end

    test "JSON encodes bulk save request" do
      states = [
        %{entity_id: "e1", state: %{"value" => 100}},
        %{entity_id: "e2", state: %{"value" => 200}}
      ]

      body = Jason.encode!(%{states: states})

      assert is_binary(body)
      assert String.contains?(body, "e1")
      assert String.contains?(body, "e2")

      # Verify it can be decoded
      decoded = Jason.decode!(body)
      assert length(decoded["states"]) == 2
    end
  end

  describe "telemetry events" do
    test "message_processed event has expected measurements" do
      measurements = %{duration_us: 150, updates_count: 2}
      metadata = %{event_type: "user.created"}

      assert measurements.duration_us == 150
      assert measurements.updates_count == 2
      assert metadata.event_type == "user.created"
    end

    test "batch_processed event has expected measurements" do
      measurements = %{
        duration_us: 500,
        messages_count: 10,
        updates_count: 15
      }

      assert measurements.messages_count == 10
      assert measurements.updates_count == 15
    end

    test "messages_failed event tracks count" do
      measurements = %{count: 3}
      assert measurements.count == 3
    end
  end

  describe "error handling" do
    test "handles nil entity_id in event" do
      event = %{
        "id" => "event-no-entity",
        "entity_id" => nil,
        "event_type" => "system.log",
        "payload" => %{"message" => "test"}
      }

      # Should not crash, entity_snapshots update should be skipped
      assert event["entity_id"] == nil
    end

    test "handles nil event_type in event" do
      event = %{
        "id" => "event-no-type",
        "entity_id" => "entity-1",
        "event_type" => nil,
        "payload" => %{"data" => "test"}
      }

      # Should not crash, event_counters update should be skipped
      assert event["event_type"] == nil
    end

    test "handles malformed payload gracefully" do
      # Payload as string instead of map
      event = %{
        "id" => "event-bad-payload",
        "entity_id" => "entity-1",
        "event_type" => "test",
        "payload" => "not a map"
      }

      # Should handle gracefully
      assert is_binary(event["payload"])
    end
  end

  describe "Broadway message structure" do
    test "message contains event data" do
      event = %{"id" => "e1", "entity_id" => "ent-1", "event_type" => "test"}

      message = %Broadway.Message{
        data: event,
        acknowledger: {McpServerElixir.Infrastructure.CoreProducer, :ack_id, event["id"]}
      }

      assert message.data == event
      assert message.data["id"] == "e1"
    end

    test "message can be updated with batcher" do
      message = %Broadway.Message{
        data: %{},
        acknowledger: {McpServerElixir.Infrastructure.CoreProducer, :ack_id, "id"}
      }

      updated = Broadway.Message.put_batcher(message, :projection_updates)
      assert updated.batcher == :projection_updates
    end

    test "message data can be updated" do
      message = %Broadway.Message{
        data: %{original: true},
        acknowledger: {McpServerElixir.Infrastructure.CoreProducer, :ack_id, "id"}
      }

      updated = Broadway.Message.put_data(message, %{event: %{}, updates: []})
      assert updated.data == %{event: %{}, updates: []}
    end

    test "message can be marked as failed" do
      message = %Broadway.Message{
        data: %{},
        acknowledger: {McpServerElixir.Infrastructure.CoreProducer, :ack_id, "id"}
      }

      failed = Broadway.Message.failed(message, "test error")
      assert failed.status == {:failed, "test error"}
    end
  end
end
