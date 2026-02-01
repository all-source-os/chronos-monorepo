defmodule QueryServiceEx.CircuitBreakerTest do
  @moduledoc """
  Tests for the CircuitBreaker GenServer.

  Verifies circuit breaker state transitions, failure tracking,
  recovery logic, and telemetry emission.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.CircuitBreaker

  # Helper to wait for circuit state with retries (handles CI timing variability)
  defp wait_for_state(server, circuit, expected_state, max_attempts \\ 20) do
    Enum.reduce_while(1..max_attempts, :timeout, fn attempt, _acc ->
      actual_state = CircuitBreaker.get_state(circuit, server)

      if actual_state == expected_state do
        {:halt, :ok}
      else
        # Progressive backoff: start at 50ms, increase for later attempts
        sleep_time = min(50 * attempt, 500)
        Process.sleep(sleep_time)
        {:cont, :timeout}
      end
    end)
  end

  setup do
    # Start a new circuit breaker for each test with low thresholds for testing
    {:ok, pid} =
      CircuitBreaker.start_link(
        name: :"circuit_breaker_test_#{System.unique_integer([:positive])}",
        failure_threshold: 3,
        reset_timeout_ms: 100,
        half_open_max_calls: 2
      )

    {:ok, server: pid}
  end

  describe "start_link/1" do
    test "starts with default name when not specified" do
      # This uses the module name as default, we just verify it starts
      {:ok, pid} =
        CircuitBreaker.start_link(
          name: :"circuit_breaker_default_test_#{System.unique_integer([:positive])}"
        )

      assert Process.alive?(pid)
    end

    test "starts with custom configuration" do
      {:ok, pid} =
        CircuitBreaker.start_link(
          name: :"circuit_breaker_custom_#{System.unique_integer([:positive])}",
          failure_threshold: 10,
          reset_timeout_ms: 5000,
          half_open_max_calls: 5
        )

      assert Process.alive?(pid)
    end
  end

  describe "get_state/2" do
    test "returns :closed for new circuits", %{server: server} do
      assert CircuitBreaker.get_state(:test_circuit, server) == :closed
    end

    test "returns :closed for default circuit", %{server: server} do
      assert CircuitBreaker.get_state(:default, server) == :closed
    end
  end

  describe "get_stats/2" do
    test "returns initial stats for new circuit", %{server: server} do
      stats = CircuitBreaker.get_stats(:test_circuit, server)

      assert stats.state == :closed
      assert stats.failure_count == 0
      assert stats.success_count == 0
      assert stats.half_open_calls == 0
      assert stats.last_failure_at == nil
      assert stats.last_success_at == nil
    end
  end

  describe "call/3 in closed state" do
    test "executes function and returns result on success", %{server: server} do
      result = CircuitBreaker.call(:test_circuit, fn -> {:ok, "success"} end, server: server)
      assert result == {:ok, "success"}
    end

    test "passes through :ok result", %{server: server} do
      result = CircuitBreaker.call(:test_circuit, fn -> :ok end, server: server)
      assert result == :ok
    end

    test "records success and resets failure count", %{server: server} do
      # Cause a failure first
      CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 1

      # Now succeed
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "ok"} end, server: server)

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 0
      assert stats.success_count == 1
    end

    test "records failure on error result", %{server: server} do
      CircuitBreaker.call(:test_circuit, fn -> {:error, :something_wrong} end, server: server)

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 1
      assert stats.last_failure_at != nil
    end

    test "opens circuit after threshold failures", %{server: server} do
      # Cause 3 failures (threshold is 3)
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      assert CircuitBreaker.get_state(:test_circuit, server) == :open
    end

    test "captures exceptions as failures", %{server: server} do
      result =
        CircuitBreaker.call(:test_circuit, fn -> raise "boom" end, server: server)

      assert {:error, {:exception, %RuntimeError{message: "boom"}}} = result

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 1
    end

    test "captures throws as failures", %{server: server} do
      result =
        CircuitBreaker.call(:test_circuit, fn -> throw(:oops) end, server: server)

      assert {:error, {:throw, :oops}} = result

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 1
    end
  end

  describe "call/3 in open state" do
    setup %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      :ok
    end

    test "rejects calls immediately", %{server: server} do
      result =
        CircuitBreaker.call(:test_circuit, fn -> {:ok, "should not run"} end, server: server)

      assert result == {:error, :circuit_open}
    end

    test "does not execute the function", %{server: server} do
      test_pid = self()

      CircuitBreaker.call(
        :test_circuit,
        fn ->
          send(test_pid, :function_called)
          {:ok, "result"}
        end,
        server: server
      )

      refute_receive :function_called
    end
  end

  describe "state transitions" do
    test "transitions from closed to open after threshold", %{server: server} do
      assert CircuitBreaker.get_state(:test_circuit, server) == :closed

      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      assert CircuitBreaker.get_state(:test_circuit, server) == :open
    end

    test "transitions from open to half-open after timeout", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      assert CircuitBreaker.get_state(:test_circuit, server) == :open

      # Wait for reset timeout with retries to handle CI timing variability
      assert :ok = wait_for_state(server, :test_circuit, :half_open),
             "Circuit should transition to half_open after timeout"
    end

    test "transitions from half-open to closed after enough successes", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      # Wait for half-open with retries to handle CI timing variability
      assert :ok = wait_for_state(server, :test_circuit, :half_open),
             "Circuit should transition to half_open"

      # 2 successful calls needed (half_open_max_calls in setup)
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "success1"} end, server: server)
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "success2"} end, server: server)

      assert CircuitBreaker.get_state(:test_circuit, server) == :closed
    end

    test "transitions from half-open back to open on failure", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      # Wait for half-open with retries to handle CI timing variability
      assert :ok = wait_for_state(server, :test_circuit, :half_open),
             "Circuit should transition to half_open"

      # One failure reopens the circuit
      CircuitBreaker.call(:test_circuit, fn -> {:error, :fail_again} end, server: server)

      assert CircuitBreaker.get_state(:test_circuit, server) == :open
    end
  end

  describe "reset/2" do
    test "manually resets circuit to closed", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      assert CircuitBreaker.get_state(:test_circuit, server) == :open

      # Reset
      :ok = CircuitBreaker.reset(:test_circuit, server)

      assert CircuitBreaker.get_state(:test_circuit, server) == :closed

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.failure_count == 0
    end
  end

  describe "telemetry events" do
    setup do
      ref = make_ref()
      test_pid = self()

      events = [
        [:query_service_ex, :circuit_breaker, :opened],
        [:query_service_ex, :circuit_breaker, :closed],
        [:query_service_ex, :circuit_breaker, :half_open],
        [:query_service_ex, :circuit_breaker, :rejected]
      ]

      for event <- events do
        :telemetry.attach(
          "test-circuit-#{inspect(ref)}-#{inspect(event)}",
          event,
          fn event_name, measurements, metadata, _config ->
            send(test_pid, {:telemetry_event, event_name, measurements, metadata})
          end,
          nil
        )
      end

      on_exit(fn ->
        for event <- events do
          :telemetry.detach("test-circuit-#{inspect(ref)}-#{inspect(event)}")
        end
      end)

      :ok
    end

    test "emits :opened when circuit opens from closed", %{server: server} do
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :opened],
                      %{failure_count: 3}, %{from_state: :closed}}
    end

    test "emits :rejected when call is rejected", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      # Clear the :opened event
      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :opened], _, _}

      # Attempt a call on open circuit
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "test"} end, server: server)

      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :rejected], %{},
                      %{circuit: :test_circuit}}
    end

    test "emits :half_open when transitioning to half-open", %{server: server} do
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      # Clear the :opened event
      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :opened], _, _}

      # Wait for half-open transition with retries to handle CI timing variability
      assert :ok = wait_for_state(server, :test_circuit, :half_open),
             "Circuit should transition to half_open"

      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :half_open], %{},
                      %{circuit: :test_circuit}},
                     1000
    end

    test "emits :closed when circuit recovers", %{server: server} do
      # Open the circuit
      for _ <- 1..3 do
        CircuitBreaker.call(:test_circuit, fn -> {:error, :fail} end, server: server)
      end

      # Clear the :opened event
      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :opened], _, _}

      # Wait for half-open with retries to handle CI timing variability
      assert :ok = wait_for_state(server, :test_circuit, :half_open),
             "Circuit should transition to half_open"

      # Successful calls to close
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "s1"} end, server: server)
      CircuitBreaker.call(:test_circuit, fn -> {:ok, "s2"} end, server: server)

      assert_receive {:telemetry_event, [:query_service_ex, :circuit_breaker, :closed], %{}, %{}},
                     1000
    end
  end

  describe "multiple circuits" do
    test "circuits are independent", %{server: server} do
      # Open circuit A
      for _ <- 1..3 do
        CircuitBreaker.call(:circuit_a, fn -> {:error, :fail} end, server: server)
      end

      assert CircuitBreaker.get_state(:circuit_a, server) == :open
      assert CircuitBreaker.get_state(:circuit_b, server) == :closed

      # Circuit B should still work
      result = CircuitBreaker.call(:circuit_b, fn -> {:ok, "success"} end, server: server)
      assert result == {:ok, "success"}
    end
  end

  describe "unexpected return values" do
    test "treats non-error tuples as success", %{server: server} do
      result = CircuitBreaker.call(:test_circuit, fn -> "just a string" end, server: server)
      assert result == "just a string"

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.success_count == 1
      assert stats.failure_count == 0
    end

    test "treats maps as success", %{server: server} do
      result = CircuitBreaker.call(:test_circuit, fn -> %{data: "value"} end, server: server)
      assert result == %{data: "value"}

      stats = CircuitBreaker.get_stats(:test_circuit, server)
      assert stats.success_count == 1
    end
  end
end
