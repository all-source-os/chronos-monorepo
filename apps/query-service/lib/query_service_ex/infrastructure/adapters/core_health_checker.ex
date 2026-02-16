defmodule QueryServiceEx.Infrastructure.Adapters.CoreHealthChecker do
  @moduledoc """
  GenServer that periodically polls GET /health on each Core read URL.

  Tracks healthy/unhealthy status and replication_lag_ms per node in an ETS
  table for fast concurrent reads. RustCoreClient checks this state before
  selecting a read node, skipping unhealthy or lagging followers.

  ## Configuration

      CORE_HEALTH_CHECK_INTERVAL_MS  — polling interval (default: 5000)
      CORE_MAX_REPLICATION_LAG_MS    — max acceptable lag (default: 5000)
  """

  use GenServer

  require Logger

  @table :core_node_health
  @default_interval_ms 5_000
  @health_timeout_ms 3_000

  # -- Public API --

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "Returns the list of healthy read URLs (lag below threshold)."
  def healthy_read_urls do
    max_lag = max_replication_lag_ms()

    case :ets.tab2list(@table) do
      [] ->
        # Table empty (checker hasn't run yet) — return all configured URLs
        configured_read_urls()

      entries ->
        entries
        |> Enum.filter(fn {_url, status, lag_ms, _last_check} ->
          status == :healthy and lag_ms <= max_lag
        end)
        |> Enum.map(fn {url, _status, _lag, _last_check} -> url end)
    end
  end

  @doc "Returns health state for all tracked nodes (for /health endpoint)."
  def node_health_states do
    case :ets.tab2list(@table) do
      [] ->
        []

      entries ->
        Enum.map(entries, fn {url, status, lag_ms, last_check} ->
          %{
            url: url,
            status: Atom.to_string(status),
            lag_ms: lag_ms,
            last_check_at: last_check
          }
        end)
    end
  end

  # -- GenServer callbacks --

  @impl true
  def init(_opts) do
    :ets.new(@table, [
      :set,
      :public,
      :named_table,
      read_concurrency: true,
      write_concurrency: true
    ])

    interval = check_interval_ms()
    schedule_check(interval)

    {:ok, %{interval: interval}}
  end

  @impl true
  def handle_info(:check_health, state) do
    urls = configured_read_urls()
    check_all(urls)
    schedule_check(state.interval)
    {:noreply, state}
  end

  # -- Private --

  defp schedule_check(interval) do
    Process.send_after(self(), :check_health, interval)
  end

  defp check_all(urls) do
    tasks =
      Enum.map(urls, fn url ->
        {url, Task.async(fn -> probe_health(url) end)}
      end)

    Enum.each(tasks, fn {url, task} ->
      now = DateTime.utc_now() |> DateTime.to_iso8601()

      case Task.yield(task, @health_timeout_ms) || Task.shutdown(task, :brutal_kill) do
        {:ok, {status, lag_ms}} ->
          :ets.insert(@table, {url, status, lag_ms, now})

        nil ->
          Logger.warning("[CoreHealthChecker] #{url} health check timed out")
          :ets.insert(@table, {url, :unhealthy, 0, now})
      end
    end)
  end

  defp probe_health(url) do
    client =
      Tesla.client(
        [
          {Tesla.Middleware.BaseUrl, url},
          Tesla.Middleware.JSON,
          {Tesla.Middleware.Timeout, timeout: @health_timeout_ms}
        ],
        {Tesla.Adapter.Hackney, [connect_options: [:inet6]]}
      )

    case Tesla.get(client, "/health") do
      {:ok, %Tesla.Env{status: 200, body: body}} when is_map(body) ->
        lag_ms = extract_replication_lag(body)
        {:healthy, lag_ms}

      {:ok, %Tesla.Env{status: status}} ->
        Logger.warning("[CoreHealthChecker] #{url} returned HTTP #{status}")
        {:unhealthy, 0}

      {:error, reason} ->
        Logger.warning("[CoreHealthChecker] #{url} unreachable: #{inspect(reason)}")
        {:unhealthy, 0}
    end
  rescue
    error ->
      Logger.warning("[CoreHealthChecker] #{url} probe error: #{inspect(error)}")
      {:unhealthy, 0}
  end

  defp extract_replication_lag(body) do
    case body do
      %{"replication" => %{"replication_lag_ms" => lag}} when is_number(lag) ->
        lag

      %{"replication" => %{"lag_ms" => lag}} when is_number(lag) ->
        lag

      _ ->
        0
    end
  end

  defp configured_read_urls do
    case Application.get_env(:query_service_ex, :core_read_urls) do
      urls when is_list(urls) and urls != [] -> urls
      _ -> [Application.get_env(:query_service_ex, :core_url, "http://localhost:3900")]
    end
  end

  defp check_interval_ms do
    Application.get_env(:query_service_ex, :core_health_check_interval_ms, @default_interval_ms)
  end

  defp max_replication_lag_ms do
    Application.get_env(:query_service_ex, :core_max_replication_lag_ms, 5_000)
  end
end
