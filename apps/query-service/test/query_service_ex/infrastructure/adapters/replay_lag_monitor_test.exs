defmodule QueryServiceEx.Infrastructure.Adapters.ReplayLagMonitorTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.ReplayLagMonitor

  describe "get_state/0" do
    test "returns default state before first poll" do
      # Start the monitor (it may already be started by the app)
      case Process.whereis(ReplayLagMonitor) do
        nil ->
          {:ok, _pid} = ReplayLagMonitor.start_link()

        _pid ->
          :ok
      end

      state = ReplayLagMonitor.get_state()

      assert is_map(state)
      assert Map.has_key?(state, :status)
      assert Map.has_key?(state, :core_event_count)
      assert Map.has_key?(state, :qs_replayed_count)
      assert Map.has_key?(state, :lag)
      assert Map.has_key?(state, :last_replayed_at)
      assert state.status in ["ready", "catching_up"]
    end

    test "response shape matches expected format" do
      state = ReplayLagMonitor.get_state()

      assert is_binary(state.status)
      assert is_integer(state.core_event_count) or state.core_event_count == 0
      assert is_integer(state.qs_replayed_count) or state.qs_replayed_count == 0
      assert is_integer(state.lag) or state.lag == 0
    end
  end
end
