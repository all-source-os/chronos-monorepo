defmodule QueryServiceEx.CircuitBreaker do
  @moduledoc """
  Circuit breaker implementation to prevent cascading failures.

  Provides protection against repeated failures to external services (Core backend).
  The circuit breaker has three states:

  - **:closed** - Normal operation. Requests pass through.
  - **:open** - Too many failures. Requests fail immediately without calling backend.
  - **:half_open** - Testing recovery. Limited requests allowed to test if backend recovered.

  ## Usage

      # Execute a function with circuit breaker protection
      case CircuitBreaker.call(:core_backend, fn -> RustCoreClient.health_check() end) do
        {:ok, result} -> result
        {:error, :circuit_open} -> # Return cached/fallback data
        {:error, reason} -> # Handle actual error
      end

  ## Configuration

      config :query_service_ex, QueryServiceEx.CircuitBreaker,
        failure_threshold: 5,        # Failures before opening circuit
        reset_timeout_ms: 30_000,    # Time before trying half-open
        half_open_max_calls: 3       # Max calls in half-open state

  """

  use GenServer
  require Logger

  @default_failure_threshold 5
  @default_reset_timeout_ms 30_000
  @default_half_open_max_calls 3

  # Client API

  @doc """
  Start the circuit breaker GenServer.
  """
  def start_link(opts \\ []) do
    name = opts[:name] || __MODULE__
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Execute a function with circuit breaker protection.

  Returns:
  - `{:ok, result}` - Function executed successfully
  - `{:error, :circuit_open}` - Circuit is open, function not called
  - `{:error, reason}` - Function returned an error

  ## Options
  - `:circuit` - Circuit name (default: :default)
  - `:timeout` - GenServer call timeout (default: 5000ms)
  """
  def call(circuit_name \\ :default, fun, opts \\ []) do
    server = opts[:server] || __MODULE__
    timeout = opts[:timeout] || 5_000

    case GenServer.call(server, {:get_state, circuit_name}, timeout) do
      :open ->
        Logger.warning("[CircuitBreaker] Circuit #{circuit_name} is open, rejecting request")

        :telemetry.execute(
          [:query_service_ex, :circuit_breaker, :rejected],
          %{},
          %{circuit: circuit_name}
        )

        {:error, :circuit_open}

      state when state in [:closed, :half_open] ->
        execute_with_tracking(server, circuit_name, fun, timeout)
    end
  end

  @doc """
  Get the current state of a circuit.
  """
  def get_state(circuit_name \\ :default, server \\ __MODULE__) do
    GenServer.call(server, {:get_state, circuit_name})
  end

  @doc """
  Get statistics for a circuit.
  """
  def get_stats(circuit_name \\ :default, server \\ __MODULE__) do
    GenServer.call(server, {:get_stats, circuit_name})
  end

  @doc """
  Manually reset a circuit to closed state.
  """
  def reset(circuit_name \\ :default, server \\ __MODULE__) do
    GenServer.call(server, {:reset, circuit_name})
  end

  # Server Callbacks

  @impl GenServer
  def init(opts) do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    state = %{
      failure_threshold:
        opts[:failure_threshold] || config[:failure_threshold] || @default_failure_threshold,
      reset_timeout_ms:
        opts[:reset_timeout_ms] || config[:reset_timeout_ms] || @default_reset_timeout_ms,
      half_open_max_calls:
        opts[:half_open_max_calls] || config[:half_open_max_calls] || @default_half_open_max_calls,
      circuits: %{}
    }

    Logger.info(
      "[CircuitBreaker] Started with threshold=#{state.failure_threshold}, reset_timeout=#{state.reset_timeout_ms}ms"
    )

    {:ok, state}
  end

  @impl GenServer
  def handle_call({:get_state, circuit_name}, _from, state) do
    circuit = get_or_init_circuit(state.circuits, circuit_name)
    {:reply, circuit.state, state}
  end

  @impl GenServer
  def handle_call({:get_stats, circuit_name}, _from, state) do
    circuit = get_or_init_circuit(state.circuits, circuit_name)

    stats = %{
      state: circuit.state,
      failure_count: circuit.failure_count,
      success_count: circuit.success_count,
      half_open_calls: circuit.half_open_calls,
      last_failure_at: circuit.last_failure_at,
      last_success_at: circuit.last_success_at
    }

    {:reply, stats, state}
  end

  @impl GenServer
  def handle_call({:reset, circuit_name}, _from, state) do
    Logger.info("[CircuitBreaker] Manually resetting circuit #{circuit_name}")
    new_circuits = Map.put(state.circuits, circuit_name, init_circuit())
    {:reply, :ok, %{state | circuits: new_circuits}}
  end

  @impl GenServer
  def handle_call({:record_success, circuit_name}, _from, state) do
    circuit = get_or_init_circuit(state.circuits, circuit_name)
    new_circuit = record_success(circuit, state)
    new_circuits = Map.put(state.circuits, circuit_name, new_circuit)
    {:reply, :ok, %{state | circuits: new_circuits}}
  end

  @impl GenServer
  def handle_call({:record_failure, circuit_name}, _from, state) do
    circuit = get_or_init_circuit(state.circuits, circuit_name)
    new_circuit = record_failure(circuit, circuit_name, state)
    new_circuits = Map.put(state.circuits, circuit_name, new_circuit)
    {:reply, :ok, %{state | circuits: new_circuits}}
  end

  @impl GenServer
  def handle_info({:reset_timeout, circuit_name}, state) do
    case Map.get(state.circuits, circuit_name) do
      %{state: :open} = circuit ->
        Logger.info("[CircuitBreaker] Transitioning circuit #{circuit_name} to half-open")

        :telemetry.execute(
          [:query_service_ex, :circuit_breaker, :half_open],
          %{},
          %{circuit: circuit_name}
        )

        new_circuit = %{circuit | state: :half_open, half_open_calls: 0}
        new_circuits = Map.put(state.circuits, circuit_name, new_circuit)
        {:noreply, %{state | circuits: new_circuits}}

      _ ->
        {:noreply, state}
    end
  end

  # Private Functions

  defp execute_with_tracking(server, circuit_name, fun, timeout) do
    case fun.() do
      {:ok, result} ->
        GenServer.call(server, {:record_success, circuit_name}, timeout)
        {:ok, result}

      :ok ->
        GenServer.call(server, {:record_success, circuit_name}, timeout)
        :ok

      {:error, reason} ->
        GenServer.call(server, {:record_failure, circuit_name}, timeout)
        {:error, reason}

      other ->
        # Treat unexpected return as success
        GenServer.call(server, {:record_success, circuit_name}, timeout)
        other
    end
  rescue
    error ->
      GenServer.call(server, {:record_failure, circuit_name}, timeout)
      {:error, {:exception, error}}
  catch
    kind, reason ->
      GenServer.call(server, {:record_failure, circuit_name}, timeout)
      {:error, {kind, reason}}
  end

  defp get_or_init_circuit(circuits, name) do
    Map.get(circuits, name, init_circuit())
  end

  defp init_circuit do
    %{
      state: :closed,
      failure_count: 0,
      success_count: 0,
      half_open_calls: 0,
      last_failure_at: nil,
      last_success_at: nil
    }
  end

  defp record_success(circuit, config) do
    new_circuit = %{
      circuit
      | success_count: circuit.success_count + 1,
        last_success_at: DateTime.utc_now()
    }

    case circuit.state do
      :half_open ->
        new_half_open_calls = circuit.half_open_calls + 1

        if new_half_open_calls >= config.half_open_max_calls do
          # Enough successful calls in half-open, close the circuit
          Logger.info("[CircuitBreaker] Circuit recovered, transitioning to closed")

          :telemetry.execute(
            [:query_service_ex, :circuit_breaker, :closed],
            %{},
            %{}
          )

          %{new_circuit | state: :closed, failure_count: 0, half_open_calls: 0}
        else
          %{new_circuit | half_open_calls: new_half_open_calls}
        end

      _ ->
        # In closed state, just reset failure count on success
        %{new_circuit | failure_count: 0}
    end
  end

  defp record_failure(circuit, circuit_name, config) do
    new_failure_count = circuit.failure_count + 1

    new_circuit = %{
      circuit
      | failure_count: new_failure_count,
        last_failure_at: DateTime.utc_now()
    }

    case circuit.state do
      :half_open ->
        # Any failure in half-open reopens the circuit
        Logger.warning("[CircuitBreaker] Failure in half-open state, reopening circuit")
        schedule_reset_timeout(circuit_name, config.reset_timeout_ms)

        :telemetry.execute(
          [:query_service_ex, :circuit_breaker, :opened],
          %{failure_count: new_failure_count},
          %{from_state: :half_open}
        )

        %{new_circuit | state: :open, half_open_calls: 0}

      :closed ->
        if new_failure_count >= config.failure_threshold do
          Logger.warning(
            "[CircuitBreaker] Failure threshold reached (#{new_failure_count}), opening circuit"
          )

          schedule_reset_timeout(circuit_name, config.reset_timeout_ms)

          :telemetry.execute(
            [:query_service_ex, :circuit_breaker, :opened],
            %{failure_count: new_failure_count},
            %{from_state: :closed}
          )

          %{new_circuit | state: :open}
        else
          new_circuit
        end

      :open ->
        new_circuit
    end
  end

  defp schedule_reset_timeout(circuit_name, timeout_ms) do
    Process.send_after(self(), {:reset_timeout, circuit_name}, timeout_ms)
  end
end
