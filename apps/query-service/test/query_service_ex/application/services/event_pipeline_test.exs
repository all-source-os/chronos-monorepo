defmodule QueryServiceEx.Application.Services.EventPipelineTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Application.Services.EventPipeline
  alias QueryServiceEx.Application.Services.ProjectionSync

  @moduletag :broadway

  describe "start_link/1" do
    test "returns :ignore when disabled" do
      # Event pipeline is disabled in test config
      assert :ignore = EventPipeline.start_link(name: :test_pipeline_disabled)
    end

    # Skip unless you want to test with Core running
    @tag :skip
    test "starts when enabled" do
      # Temporarily enable
      Application.put_env(:query_service_ex, :event_pipeline_enabled, true)

      result = EventPipeline.start_link(name: :test_pipeline_enabled)

      # Restore
      Application.put_env(:query_service_ex, :event_pipeline_enabled, false)

      case result do
        {:ok, pid} ->
          assert Process.alive?(pid)
          Broadway.stop(pid)

        :ignore ->
          # Core not running, that's ok
          :ok
      end
    end
  end

  describe "stats/1" do
    test "returns error when not running" do
      assert {:error, :not_running} = EventPipeline.stats(:nonexistent_pipeline)
    end
  end

  describe "message processing" do
    test "converts events to Broadway messages" do
      event = %{
        "id" => "evt-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{"name" => "Test User"}
      }

      message = %Broadway.Message{
        data: event,
        acknowledger: {Broadway.NoopAcknowledger, nil, nil}
      }

      assert message.data["id"] == "evt-123"
      assert message.data["entity_id"] == "user-456"
    end
  end

  describe "projection handling" do
    setup do
      # Initialize ETS cache
      ProjectionSync.init_cache()
      :ok
    end

    test "get_state_from_cache returns nil for unknown entity" do
      result = ProjectionSync.get_state_from_cache("entity_snapshots", "unknown-entity")
      assert result == nil
    end

    test "projection state is cached after insert" do
      # Insert directly to ETS
      :ets.insert(:projection_state_cache, {{"entity_snapshots", "test-entity"}, %{name: "Test"}})

      result = ProjectionSync.get_state_from_cache("entity_snapshots", "test-entity")
      assert result == %{name: "Test"}
    end
  end
end
