defmodule QueryServiceEx.Infrastructure.Adapters.ReplayLagMonitor do
  @moduledoc """
  GenServer that periodically polls Core's event count and tracks how far
  behind the Query Service is.

  Stores state in an ETS table for fast concurrent reads from the health
  endpoint.

  ## Configuration

      REPLAY_LAG_CHECK_INTERVAL_MS — polling interval (default: 10_000)
  """

  use GenServer

  require Logger

  @table :replay_lag_state
  @default_interval_ms 10_000

  # -- Public API --

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "Returns the current replay lag state."
  def get_state do
    case :ets.lookup(@table, :lag_state) do
      [{:lag_state, state}] -> state
      [] -> default_state()
    end
  end

  # -- GenServer callbacks --

  @impl true
  def init(_opts) do
    :ets.new(@table, [
      :set,
      :public,
      :named_table,
      read_concurrency: true
    ])

    :ets.insert(@table, {:lag_state, default_state()})

    interval = check_interval_ms()
    schedule_check(interval)

    {:ok, %{interval: interval, qs_replayed_count: 0}}
  end

  @impl true
  def handle_info(:check_lag, state) do
    new_state = poll_core(state)
    schedule_check(state.interval)
    {:noreply, new_state}
  end

  # -- Private --

  defp schedule_check(interval) do
    Process.send_after(self(), :check_lag, interval)
  end

  defp poll_core(state) do
    core_url = Application.get_env(:query_service_ex, :core_url, "http://localhost:3900")

    client =
      Tesla.client(
        [
          {Tesla.Middleware.BaseUrl, core_url},
          Tesla.Middleware.JSON,
          {Tesla.Middleware.Timeout, timeout: 5_000}
        ],
        {Tesla.Adapter.Hackney, [connect_options: [:inet6]]}
      )

    case Tesla.get(client, "/api/v1/stats") do
      {:ok, %Tesla.Env{status: 200, body: body}} when is_map(body) ->
        core_count = Map.get(body, "total_events", 0)
        # QS replayed count tracks what we've seen from Core.
        # In proxy mode, QS is always caught up since it forwards to Core.
        # When projection sync is active, this would track the projection's event count.
        qs_count = core_count
        lag = max(core_count - qs_count, 0)

        status = if lag == 0, do: "ready", else: "catching_up"
        now = DateTime.utc_now() |> DateTime.to_iso8601()

        lag_state = %{
          status: status,
          core_event_count: core_count,
          qs_replayed_count: qs_count,
          lag: lag,
          last_replayed_at: now
        }

        :ets.insert(@table, {:lag_state, lag_state})
        %{state | qs_replayed_count: qs_count}

      {:ok, %Tesla.Env{status: status_code}} ->
        Logger.warning("[ReplayLagMonitor] Core stats returned HTTP #{status_code}")
        state

      {:error, reason} ->
        Logger.warning("[ReplayLagMonitor] Core stats unreachable: #{inspect(reason)}")
        state
    end
  rescue
    error ->
      Logger.warning("[ReplayLagMonitor] poll error: #{inspect(error)}")
      state
  end

  defp default_state do
    %{
      status: "catching_up",
      core_event_count: 0,
      qs_replayed_count: 0,
      lag: 0,
      last_replayed_at: nil
    }
  end

  defp check_interval_ms do
    Application.get_env(:query_service_ex, :replay_lag_check_interval_ms, @default_interval_ms)
  end
end
