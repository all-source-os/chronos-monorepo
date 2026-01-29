defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient do
  @moduledoc """
  WebSocket client for subscribing to real-time events from Rust Core.

  This GenServer connects to Core's `/api/v1/events/stream` WebSocket endpoint
  and broadcasts received events via Phoenix.PubSub for local consumption.

  ## Features
  - Auto-reconnect with exponential backoff
  - Event parsing and validation
  - PubSub broadcasting for local GenServers
  - Connection state tracking

  ## Topics
  - `events:all` - All events
  - `events:{entity_id}` - Events for specific entity
  - `events:type:{event_type}` - Events by type

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
  @initial_backoff_ms 1_000
  @max_backoff_ms 30_000
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
    url = opts[:url] || Application.get_env(:query_service_ex, :core_ws_url, @default_url)
    name = opts[:name] || __MODULE__

    state = %{
      url: url,
      backoff_ms: @initial_backoff_ms,
      connected: false,
      reconnect_attempts: 0,
      events_received: 0,
      last_event_at: nil
    }

    Logger.info("[CoreWebSocketClient] Connecting to #{url}")

    case WebSockex.start_link(url, __MODULE__, state, name: name) do
      {:ok, pid} ->
        {:ok, pid}

      {:error, %WebSockex.ConnError{original: reason}} ->
        Logger.warning("[CoreWebSocketClient] Failed to connect: #{inspect(reason)}, will retry")
        # Start a GenServer that will retry connection
        start_retry_loop(url, name, state)

      {:error, reason} ->
        Logger.error("[CoreWebSocketClient] Failed to start: #{inspect(reason)}")
        {:error, reason}
    end
  end

  defp start_retry_loop(url, name, initial_state) do
    # Start a simple GenServer that retries connection
    Task.start_link(fn ->
      retry_connection(url, name, initial_state, 1)
    end)

    :ignore
  end

  defp retry_connection(url, name, state, attempt) when attempt < 10 do
    backoff = min(state.backoff_ms * attempt, @max_backoff_ms)
    Logger.info("[CoreWebSocketClient] Retry #{attempt} in #{backoff}ms")
    Process.sleep(backoff)

    case WebSockex.start_link(url, __MODULE__, state, name: name) do
      {:ok, _pid} ->
        Logger.info("[CoreWebSocketClient] Connected after #{attempt} retries")
        :ok

      {:error, _reason} ->
        retry_connection(url, name, state, attempt + 1)
    end
  end

  defp retry_connection(_url, _name, _state, _attempt) do
    Logger.error("[CoreWebSocketClient] Failed to connect after 10 retries, giving up")
    :error
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
    Logger.info("[CoreWebSocketClient] Connected to Core WebSocket")

    new_state = %{
      state
      | connected: true,
        backoff_ms: @initial_backoff_ms,
        reconnect_attempts: 0
    }

    {:ok, new_state}
  end

  @impl WebSockex
  def handle_frame({:text, json}, state) do
    case Jason.decode(json) do
      {:ok, event} ->
        broadcast_event(event)

        new_state = %{
          state
          | events_received: state.events_received + 1,
            last_event_at: DateTime.utc_now()
        }

        {:ok, new_state}

      {:error, reason} ->
        Logger.warning("[CoreWebSocketClient] Failed to parse event: #{inspect(reason)}")
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
    Logger.warning("[CoreWebSocketClient] Disconnected: #{inspect(reason)}")

    new_state = %{
      state
      | connected: false,
        reconnect_attempts: state.reconnect_attempts + 1
    }

    # Exponential backoff with jitter
    backoff = calculate_backoff(new_state.backoff_ms)

    Logger.info(
      "[CoreWebSocketClient] Reconnecting in #{backoff}ms (attempt #{new_state.reconnect_attempts})"
    )

    Process.sleep(backoff)

    {:reconnect, %{new_state | backoff_ms: min(backoff * 2, @max_backoff_ms)}}
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
      events_received: state.events_received,
      last_event_at: state.last_event_at,
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

  defp calculate_backoff(base_ms) do
    # Add jitter (±20%)
    jitter = :rand.uniform() * 0.4 - 0.2
    round(base_ms * (1 + jitter))
  end
end
