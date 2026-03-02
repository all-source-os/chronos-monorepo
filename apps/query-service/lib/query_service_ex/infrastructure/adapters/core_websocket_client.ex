defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient do
  @moduledoc """
  WebSocket client for subscribing to real-time events from Rust Core.

  This GenServer manages a Mint.WebSocket connection to Core's `/api/v1/events/stream`
  WebSocket endpoint and broadcasts received events via Phoenix.PubSub.

  The GenServer always starts successfully and registers its name, then attempts
  to establish the WebSocket connection asynchronously. This ensures the health
  check finds a running process even while the connection is being established.

  ## Features
  - Auto-reconnect with exponential backoff and jitter
  - Infinite retry on initial connection (no max attempts for startup)
  - Configurable max reconnection attempts for post-connection drops
  - Event parsing and validation
  - PubSub broadcasting for local GenServers
  - Connection state tracking

  ## Topics
  - `events:all` - All events
  - `events:{entity_id}` - Events for specific entity
  - `events:type:{event_type}` - Events by type

  ## Configuration

      config :query_service_ex,
        core_ws_url: "ws://localhost:3900/api/v1/events/stream",
        core_ws_enabled: true,
        core_ws_max_reconnect_attempts: 10,
        core_ws_initial_backoff_ms: 1_000,
        core_ws_max_backoff_ms: 30_000

  ## Example

      # Subscribe to all events
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

      # Subscribe to specific entity
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:user-123")

      # Receive events
      def handle_info({:new_event, event}, state) do
        IO.inspect(event, label: "Received event")
        {:noreply, state}
      end
  """

  use GenServer
  require Logger

  @default_url "ws://localhost:3900/api/v1/events/stream"
  @default_initial_backoff_ms 1_000
  @default_max_backoff_ms 30_000
  @default_max_reconnect_attempts 10
  @pubsub QueryServiceEx.PubSub

  # Client API

  @doc """
  Start the WebSocket client.

  ## Options
    * `:url` - WebSocket URL (default: ws://localhost:3900/api/v1/events/stream)
    * `:name` - Process name (default: __MODULE__)
    * `:enabled` - Whether to start the client (default: true in prod, false in test)
  """
  def start_link(opts \\ []) do
    enabled = opts[:enabled] || Application.get_env(:query_service_ex, :core_ws_enabled, true)

    if enabled do
      name = opts[:name] || __MODULE__
      GenServer.start_link(__MODULE__, opts, name: name)
    else
      Logger.info("[CoreWebSocketClient] Disabled, not connecting")
      :ignore
    end
  end

  @doc """
  Get the current connection status.
  """
  def status(pid \\ __MODULE__) do
    GenServer.call(pid, :get_status, 5_000)
  catch
    :exit, _ -> {:error, :not_running}
  end

  @doc """
  Get statistics about the WebSocket connection.
  """
  def stats(pid \\ __MODULE__) do
    GenServer.call(pid, :get_stats, 5_000)
  catch
    :exit, _ -> {:error, :not_running}
  end

  # GenServer Callbacks

  @impl GenServer
  def init(opts) do
    config = build_config(opts)
    extra_headers = build_auth_headers()

    state = %{
      url: config.url,
      backoff_ms: config.initial_backoff,
      initial_backoff_ms: config.initial_backoff,
      max_backoff_ms: config.max_backoff,
      max_reconnect_attempts: config.max_attempts,
      extra_headers: extra_headers,
      connected: false,
      ws_pid: nil,
      reconnect_attempts: 0,
      total_reconnects: 0,
      events_received: 0,
      last_event_at: nil,
      last_error: nil,
      started_at: DateTime.utc_now()
    }

    Logger.info("[CoreWebSocketClient] Starting, will connect to #{config.url}")

    # Schedule immediate connection attempt
    send(self(), :connect)

    {:ok, state}
  end

  @impl GenServer
  def handle_call(:get_status, _from, state) do
    status = if state.connected, do: :connected, else: :disconnected
    {:reply, status, state}
  end

  @impl GenServer
  def handle_call(:get_stats, _from, state) do
    stats = %{
      connected: state.connected,
      reconnect_attempts: state.reconnect_attempts,
      total_reconnects: state.total_reconnects,
      max_reconnect_attempts: state.max_reconnect_attempts,
      events_received: state.events_received,
      last_event_at: state.last_event_at,
      last_error: state.last_error,
      started_at: state.started_at,
      url: state.url
    }

    {:reply, stats, state}
  end

  @impl GenServer
  def handle_info(:connect, state) do
    case attempt_ws_connection(state) do
      {:ok, ws_pid} ->
        Process.monitor(ws_pid)

        Logger.info("[CoreWebSocketClient] Connected to Core WebSocket", url: state.url)

        :telemetry.execute(
          [:query_service_ex, :websocket, :connected],
          %{reconnect_attempts: state.reconnect_attempts},
          %{url: state.url}
        )

        {:noreply,
         %{
           state
           | connected: true,
             ws_pid: ws_pid,
             backoff_ms: state.initial_backoff_ms,
             reconnect_attempts: 0,
             last_error: nil
         }}

      {:error, reason} ->
        new_attempts = state.reconnect_attempts + 1
        backoff = calculate_backoff(state.backoff_ms, state.max_backoff_ms)

        Logger.warning(
          "[CoreWebSocketClient] Connection failed: #{inspect(reason)}, retry #{new_attempts} in #{backoff}ms"
        )

        :telemetry.execute(
          [:query_service_ex, :websocket, :reconnect_attempt],
          %{attempt: new_attempts, backoff_ms: backoff},
          %{url: state.url}
        )

        Process.send_after(self(), :connect, backoff)

        {:noreply,
         %{
           state
           | reconnect_attempts: new_attempts,
             backoff_ms: min(backoff * 2, state.max_backoff_ms),
             last_error: reason
         }}
    end
  end

  @impl GenServer
  def handle_info({:DOWN, _ref, :process, pid, reason}, %{ws_pid: pid} = state) do
    new_total = state.total_reconnects + 1

    Logger.warning(
      "[CoreWebSocketClient] WebSocket process died: #{inspect(reason)}, will reconnect",
      reason: inspect(reason),
      total_reconnects: new_total
    )

    :telemetry.execute(
      [:query_service_ex, :websocket, :disconnected],
      %{reconnect_attempts: 0},
      %{url: state.url, reason: reason}
    )

    # Schedule reconnection
    backoff = calculate_backoff(state.initial_backoff_ms, state.max_backoff_ms)
    Process.send_after(self(), :connect, backoff)

    {:noreply,
     %{
       state
       | connected: false,
         ws_pid: nil,
         total_reconnects: new_total,
         backoff_ms: state.initial_backoff_ms,
         reconnect_attempts: 0,
         last_error: reason
     }}
  end

  @impl GenServer
  def handle_info({:websocket_event, event}, state) do
    broadcast_event(event)

    new_state = %{
      state
      | events_received: state.events_received + 1,
        last_event_at: DateTime.utc_now()
    }

    {:noreply, new_state}
  end

  @impl GenServer
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  # Private Functions

  defp attempt_ws_connection(state) do
    parent = self()

    # IPv6 is handled by Mint via transport_opts: [:inet6] in the worker.
    # This resolves Fly.io .internal hostnames (AAAA records only).
    ws_opts = [
      extra_headers: state.extra_headers,
      socket_connect_timeout: 10_000
    ]

    case QueryServiceEx.Infrastructure.Adapters.CoreWebSocketWorker.start(
           state.url,
           parent,
           state,
           ws_opts
         ) do
      {:ok, pid} -> {:ok, pid}
      {:error, reason} -> {:error, reason}
    end
  end

  defp build_config(opts) do
    %{
      url: opts[:url] || Application.get_env(:query_service_ex, :core_ws_url, @default_url),
      initial_backoff:
        opts[:initial_backoff_ms] ||
          Application.get_env(
            :query_service_ex,
            :core_ws_initial_backoff_ms,
            @default_initial_backoff_ms
          ),
      max_backoff:
        opts[:max_backoff_ms] ||
          Application.get_env(:query_service_ex, :core_ws_max_backoff_ms, @default_max_backoff_ms),
      max_attempts:
        opts[:max_reconnect_attempts] ||
          Application.get_env(
            :query_service_ex,
            :core_ws_max_reconnect_attempts,
            @default_max_reconnect_attempts
          )
    }
  end

  defp build_auth_headers do
    case Application.get_env(:query_service_ex, :core_api_key) do
      key when key in [nil, ""] -> []
      api_key -> [{"Authorization", api_key}]
    end
  end

  defp calculate_backoff(base_ms, max_ms) do
    jitter = :rand.uniform() * 0.4 - 0.2
    backoff = round(base_ms * (1 + jitter))
    min(backoff, max_ms)
  end

  defp broadcast_event(event) do
    Phoenix.PubSub.broadcast(@pubsub, "events:all", {:new_event, event})

    if entity_id = event["entity_id"] do
      Phoenix.PubSub.broadcast(@pubsub, "events:#{entity_id}", {:new_event, event})
    end

    if event_type = event["event_type"] do
      Phoenix.PubSub.broadcast(@pubsub, "events:type:#{event_type}", {:new_event, event})
    end

    Logger.debug("[CoreWebSocketClient] Broadcast event: #{event["id"] || "unknown"}")
  end
end
