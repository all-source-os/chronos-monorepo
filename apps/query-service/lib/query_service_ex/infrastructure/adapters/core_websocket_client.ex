defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient do
  @moduledoc """
  WebSocket client for subscribing to real-time events from Rust Core.

  This GenServer connects to Core's `/api/v1/events/stream` WebSocket endpoint
  and broadcasts received events via Phoenix.PubSub for local consumption.

  ## Features
  - Auto-reconnect with exponential backoff and jitter
  - Configurable max reconnection attempts
  - Event parsing and validation
  - PubSub broadcasting for local GenServers
  - Connection state tracking
  - Graceful degradation on persistent failures

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

  use WebSockex
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
      do_start_link(opts)
    else
      Logger.info("[CoreWebSocketClient] Disabled, not connecting")
      :ignore
    end
  end

  defp do_start_link(opts) do
    config = build_config(opts)
    extra_headers = build_auth_headers()
    state = build_initial_state(config, extra_headers)

    Logger.info("[CoreWebSocketClient] Connecting to #{config.url}")

    attempt_connection(config.url, config.name, extra_headers, state)
  end

  defp build_config(opts) do
    %{
      url: opts[:url] || Application.get_env(:query_service_ex, :core_ws_url, @default_url),
      name: opts[:name] || __MODULE__,
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

  defp build_initial_state(config, extra_headers) do
    %{
      url: config.url,
      backoff_ms: config.initial_backoff,
      initial_backoff_ms: config.initial_backoff,
      max_backoff_ms: config.max_backoff,
      max_reconnect_attempts: config.max_attempts,
      extra_headers: extra_headers,
      connected: false,
      reconnect_attempts: 0,
      total_reconnects: 0,
      events_received: 0,
      last_event_at: nil,
      last_error: nil,
      started_at: DateTime.utc_now()
    }
  end

  defp attempt_connection(url, name, extra_headers, state) do
    # Enable IPv6 for Fly.io .internal DNS (resolves to fdaa:: addresses)
    ws_opts = [
      name: name,
      extra_headers: extra_headers,
      socket_connect_timeout: 10_000,
      conn_opts: [transport_opts: [:inet6]]
    ]

    case WebSockex.start_link(url, __MODULE__, state, ws_opts) do
      {:ok, pid} ->
        {:ok, pid}

      {:error, %WebSockex.ConnError{original: reason}} ->
        Logger.warning("[CoreWebSocketClient] Connection error: #{inspect(reason)}, will retry")
        start_retry_loop(url, name, state)

      {:error, %WebSockex.RequestError{code: code, message: msg}} ->
        # HTTP error during WebSocket handshake (e.g., 404 Not Found)
        # This usually means Core is up but the endpoint path is wrong
        Logger.warning(
          "[CoreWebSocketClient] WebSocket handshake failed: #{code} #{msg}, will retry"
        )

        start_retry_loop(url, name, state)

      {:error, reason} ->
        # Other errors - still retry instead of crashing
        Logger.warning("[CoreWebSocketClient] Failed to start: #{inspect(reason)}, will retry")
        start_retry_loop(url, name, state)
    end
  end

  defp start_retry_loop(url, name, initial_state) do
    Task.start_link(fn ->
      retry_connection(url, name, initial_state, 1)
    end)

    :ignore
  end

  defp retry_connection(url, name, state, attempt) when attempt <= state.max_reconnect_attempts do
    backoff = calculate_backoff(state.initial_backoff_ms * attempt, state.max_backoff_ms)

    Logger.info(
      "[CoreWebSocketClient] Retry #{attempt}/#{state.max_reconnect_attempts} in #{backoff}ms"
    )

    :telemetry.execute(
      [:query_service_ex, :websocket, :reconnect_attempt],
      %{attempt: attempt, backoff_ms: backoff},
      %{url: url}
    )

    Process.sleep(backoff)

    extra_headers = Map.get(state, :extra_headers, [])

    case WebSockex.start_link(url, __MODULE__, %{state | reconnect_attempts: attempt},
           name: name,
           extra_headers: extra_headers,
           socket_connect_timeout: 10_000,
           conn_opts: [transport_opts: [:inet6]]
         ) do
      {:ok, _pid} ->
        Logger.info("[CoreWebSocketClient] Connected after #{attempt} retries")

        :telemetry.execute(
          [:query_service_ex, :websocket, :reconnect_success],
          %{attempts: attempt},
          %{url: url}
        )

        :ok

      {:error, reason} ->
        Logger.warning("[CoreWebSocketClient] Retry #{attempt} failed: #{inspect(reason)}")
        retry_connection(url, name, %{state | last_error: reason}, attempt + 1)
    end
  end

  defp retry_connection(url, _name, state, attempt) do
    Logger.error(
      "[CoreWebSocketClient] Failed to connect after #{attempt - 1} retries, entering degraded mode",
      url: url,
      last_error: inspect(state.last_error)
    )

    :telemetry.execute(
      [:query_service_ex, :websocket, :reconnect_exhausted],
      %{attempts: attempt - 1},
      %{url: url, last_error: state.last_error}
    )

    # Don't crash - just enter degraded mode where real-time updates won't work
    # The service can still function with polling or cached data
    :error
  end

  defp calculate_backoff(base_ms, max_ms) do
    # Add jitter (±20%) and cap at max
    jitter = :rand.uniform() * 0.4 - 0.2
    backoff = round(base_ms * (1 + jitter))
    min(backoff, max_ms)
  end

  @doc """
  Get the current connection status.
  """
  def status(pid \\ __MODULE__) do
    WebSockex.cast(pid, {:get_status, self()})

    receive do
      {:status, status} -> status
    after
      5_000 -> {:error, :timeout}
    end
  end

  @doc """
  Get statistics about the WebSocket connection.
  """
  def stats(pid \\ __MODULE__) do
    WebSockex.cast(pid, {:get_stats, self()})

    receive do
      {:stats, stats} -> stats
    after
      5_000 -> {:error, :timeout}
    end
  end

  # WebSockex Callbacks

  @impl WebSockex
  def handle_connect(_conn, state) do
    Logger.info("[CoreWebSocketClient] Connected to Core WebSocket", url: state.url)

    :telemetry.execute(
      [:query_service_ex, :websocket, :connected],
      %{reconnect_attempts: state.reconnect_attempts},
      %{url: state.url}
    )

    new_state = %{
      state
      | connected: true,
        backoff_ms: state.initial_backoff_ms,
        reconnect_attempts: 0,
        last_error: nil
    }

    {:ok, new_state}
  end

  @impl WebSockex
  def handle_frame({:text, json}, state) do
    case Jason.decode(json) do
      {:ok, event} ->
        broadcast_event(event)

        :telemetry.execute(
          [:query_service_ex, :websocket, :message_received],
          %{size_bytes: byte_size(json)},
          %{message_type: "event", event_type: event["event_type"]}
        )

        new_state = %{
          state
          | events_received: state.events_received + 1,
            last_event_at: DateTime.utc_now()
        }

        {:ok, new_state}

      {:error, reason} ->
        Logger.warning("[CoreWebSocketClient] Failed to parse event: #{inspect(reason)}",
          error: inspect(reason)
        )

        {:ok, state}
    end
  end

  @impl WebSockex
  def handle_frame({:binary, _data}, state) do
    Logger.debug("[CoreWebSocketClient] Received binary frame (ignored)")
    {:ok, state}
  end

  @impl WebSockex
  def handle_frame({:ping, _}, state) do
    {:reply, :pong, state}
  end

  @impl WebSockex
  def handle_frame({:pong, _}, state) do
    {:ok, state}
  end

  @impl WebSockex
  def handle_disconnect(%{reason: reason}, state) do
    new_attempts = state.reconnect_attempts + 1
    new_total = state.total_reconnects + 1

    Logger.warning(
      "[CoreWebSocketClient] Disconnected: #{inspect(reason)} (attempt #{new_attempts}/#{state.max_reconnect_attempts})",
      reason: inspect(reason),
      reconnect_attempts: new_attempts
    )

    :telemetry.execute(
      [:query_service_ex, :websocket, :disconnected],
      %{reconnect_attempts: new_attempts},
      %{url: state.url, reason: reason}
    )

    # Check if we've exceeded max reconnection attempts
    if new_attempts > state.max_reconnect_attempts do
      Logger.error(
        "[CoreWebSocketClient] Max reconnection attempts (#{state.max_reconnect_attempts}) exceeded, stopping",
        url: state.url,
        total_reconnects: new_total
      )

      :telemetry.execute(
        [:query_service_ex, :websocket, :reconnect_exhausted],
        %{attempts: new_attempts, total_reconnects: new_total},
        %{url: state.url}
      )

      # Stop reconnecting - the supervision tree will handle restart if configured
      {:ok, %{state | connected: false, last_error: reason}}
    else
      new_state = %{
        state
        | connected: false,
          reconnect_attempts: new_attempts,
          total_reconnects: new_total,
          last_error: reason
      }

      # Exponential backoff with jitter
      backoff = calculate_backoff(new_state.backoff_ms, state.max_backoff_ms)

      Logger.info(
        "[CoreWebSocketClient] Reconnecting in #{backoff}ms (attempt #{new_attempts}/#{state.max_reconnect_attempts})"
      )

      Process.sleep(backoff)

      {:reconnect, %{new_state | backoff_ms: min(backoff * 2, state.max_backoff_ms)}}
    end
  end

  @impl WebSockex
  def handle_cast({:get_status, from}, state) do
    status =
      if state.connected do
        :connected
      else
        :disconnected
      end

    send(from, {:status, status})
    {:ok, state}
  end

  @impl WebSockex
  def handle_cast({:get_stats, from}, state) do
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

    send(from, {:stats, stats})
    {:ok, state}
  end

  @impl WebSockex
  def terminate(reason, _state) do
    Logger.info("[CoreWebSocketClient] Terminating: #{inspect(reason)}")
    :ok
  end

  # Private Functions

  defp broadcast_event(event) do
    # Broadcast to all events topic
    Phoenix.PubSub.broadcast(@pubsub, "events:all", {:new_event, event})

    # Broadcast to entity-specific topic
    if entity_id = event["entity_id"] do
      Phoenix.PubSub.broadcast(@pubsub, "events:#{entity_id}", {:new_event, event})
    end

    # Broadcast to event-type topic
    if event_type = event["event_type"] do
      Phoenix.PubSub.broadcast(@pubsub, "events:type:#{event_type}", {:new_event, event})
    end

    Logger.debug("[CoreWebSocketClient] Broadcast event: #{event["id"] || "unknown"}")
  end
end
