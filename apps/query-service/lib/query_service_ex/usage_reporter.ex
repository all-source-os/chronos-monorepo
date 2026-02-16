defmodule QueryServiceEx.UsageReporter do
  @moduledoc """
  Async usage increment reporter that buffers usage counts per tenant
  and flushes them to Core periodically.

  Flushes occur every 5 seconds or when any tenant accumulates 100+ increments,
  whichever comes first. On persistent failure (3 retries with exponential backoff),
  the failed increments are logged and dropped.

  Supervised in the application tree with graceful shutdown that flushes pending
  increments before stopping.
  """

  use GenServer

  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @flush_interval 5_000
  @flush_threshold 100
  @max_retries 3
  @base_delay 2_000

  # Client API

  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Record a usage increment for a tenant. Buffered and flushed asynchronously.
  """
  def record(tenant_id, count \\ 1)
      when is_binary(tenant_id) and is_integer(count) and count > 0 do
    GenServer.cast(__MODULE__, {:record, tenant_id, count})
  end

  @doc """
  Force an immediate flush of all pending increments. Used in tests and graceful shutdown.
  """
  def flush do
    GenServer.call(__MODULE__, :flush, 30_000)
  end

  @doc """
  Returns the current buffer state. Used for testing/debugging.
  """
  def pending do
    GenServer.call(__MODULE__, :pending)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    flush_interval = Keyword.get(opts, :flush_interval, @flush_interval)
    flush_threshold = Keyword.get(opts, :flush_threshold, @flush_threshold)
    sender_fn = Keyword.get(opts, :sender_fn, &default_sender/2)
    base_delay = Keyword.get(opts, :base_delay, @base_delay)

    Process.flag(:trap_exit, true)
    schedule_flush(flush_interval)

    {:ok,
     %{
       buffer: %{},
       flush_interval: flush_interval,
       flush_threshold: flush_threshold,
       sender_fn: sender_fn,
       base_delay: base_delay
     }}
  end

  @impl true
  def handle_cast({:record, tenant_id, count}, state) do
    new_buffer = Map.update(state.buffer, tenant_id, count, &(&1 + count))
    new_state = %{state | buffer: new_buffer}

    if Map.get(new_buffer, tenant_id, 0) >= state.flush_threshold do
      {flushed_state, _} = do_flush(new_state)
      {:noreply, flushed_state}
    else
      {:noreply, new_state}
    end
  end

  @impl true
  def handle_call(:flush, _from, state) do
    {new_state, results} = do_flush(state)
    {:reply, results, new_state}
  end

  @impl true
  def handle_call(:pending, _from, state) do
    {:reply, state.buffer, state}
  end

  @impl true
  def handle_info(:flush, state) do
    {new_state, _} = do_flush(state)
    schedule_flush(new_state.flush_interval)
    {:noreply, new_state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, state) do
    if map_size(state.buffer) > 0 do
      Logger.info(
        "[UsageReporter] Flushing #{map_size(state.buffer)} pending increments on shutdown"
      )

      do_flush(state)
    end

    :ok
  end

  # Internal

  defp do_flush(%{buffer: buffer} = state) when map_size(buffer) == 0 do
    {state, :ok}
  end

  defp do_flush(%{buffer: buffer} = state) do
    results =
      Enum.map(buffer, fn {tenant_id, count} ->
        result = send_with_retry(tenant_id, count, 0, state.sender_fn, state.base_delay)

        case result do
          :ok ->
            :telemetry.execute(
              [:query_service_ex, :usage_reporter, :flushed],
              %{count: count},
              %{tenant_id: tenant_id}
            )

            :ok

          {:error, reason} ->
            Logger.error(
              "[UsageReporter] Dropped #{count} increments for tenant #{tenant_id}: #{inspect(reason)}"
            )

            :telemetry.execute(
              [:query_service_ex, :usage_reporter, :dropped],
              %{count: count},
              %{tenant_id: tenant_id}
            )

            {:error, reason}
        end
      end)

    {%{state | buffer: %{}}, results}
  end

  defp send_with_retry(tenant_id, count, attempt, sender_fn, base_delay)
       when attempt < @max_retries do
    case sender_fn.(tenant_id, count) do
      :ok ->
        :ok

      {:error, reason} ->
        delay = base_delay * Integer.pow(2, attempt)

        Logger.warning(
          "[UsageReporter] Retry #{attempt + 1}/#{@max_retries} for tenant #{tenant_id} " <>
            "(#{count} increments) in #{delay}ms: #{inspect(reason)}"
        )

        Process.sleep(delay)
        send_with_retry(tenant_id, count, attempt + 1, sender_fn, base_delay)
    end
  end

  defp send_with_retry(_tenant_id, _count, _attempt, _sender_fn, _base_delay) do
    {:error, :max_retries_exceeded}
  end

  defp default_sender(tenant_id, count) do
    case Tesla.post(
           RustCoreClient.write_client(),
           "/api/v1/tenants/#{tenant_id}/usage/increment",
           %{count: count}
         ) do
      {:ok, %Tesla.Env{status: status}} when status in 200..299 ->
        :ok

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, {:http_error, status, body}}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp schedule_flush(interval) do
    Process.send_after(self(), :flush, interval)
  end
end
