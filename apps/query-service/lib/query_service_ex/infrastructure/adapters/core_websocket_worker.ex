defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketWorker do
  @moduledoc """
  WebSocket worker using Mint + Mint.WebSocket for the connection to Core.

  Connects via Mint.HTTP (with IPv6 support for Fly.io .internal DNS),
  upgrades to WebSocket, and forwards decoded frames to the parent process.
  On disconnect, the process exits so the parent GenServer can reconnect.
  """

  use GenServer
  require Logger

  defstruct [:conn, :websocket, :ref, :parent, :url, :caller]

  def start(url, parent, _state, opts) do
    GenServer.start(__MODULE__, {url, parent, opts})
  end

  @impl GenServer
  def init({url, parent, opts}) do
    uri = URI.parse(url)
    scheme = if uri.scheme in ["wss", "https"], do: :https, else: :http
    ws_scheme = if scheme == :https, do: :wss, else: :ws
    host = uri.host || "localhost"
    port = uri.port || if(scheme == :https, do: 443, else: 80)
    path = uri.path || "/"
    extra_headers = Keyword.get(opts, :extra_headers, [])
    timeout = Keyword.get(opts, :socket_connect_timeout, 10_000)

    # Resolve hostname, preferring IPv6 for Fly.io .internal DNS (AAAA-only).
    # Mint's transport_opts must be a keyword list, so we can't pass bare :inet6.
    # Instead, resolve to an IP address and connect directly.
    {connect_address, transport_opts} = resolve_host(host, timeout)

    with {:ok, conn} <-
           Mint.HTTP.connect(scheme, connect_address, port, transport_opts: transport_opts),
         {:ok, conn, ref} <- Mint.WebSocket.upgrade(ws_scheme, conn, path, extra_headers) do
      state = %__MODULE__{conn: conn, ref: ref, parent: parent, url: url}
      {:ok, state, {:continue, :await_upgrade}}
    else
      {:error, reason} ->
        {:stop, reason}

      {:error, _conn, reason} ->
        {:stop, reason}
    end
  end

  @impl GenServer
  def handle_continue(:await_upgrade, state) do
    # Wait for the HTTP 101 upgrade response
    case receive_upgrade(state.conn, state.ref) do
      {:ok, conn, websocket} ->
        Logger.info("[CoreWebSocketWorker] WebSocket connected", url: state.url)
        {:noreply, %{state | conn: conn, websocket: websocket}}

      {:error, reason} ->
        {:stop, reason, state}
    end
  end

  @impl GenServer
  def handle_info(message, %{websocket: nil} = state) do
    # Still waiting for upgrade — shouldn't happen after handle_continue
    Logger.debug("[CoreWebSocketWorker] Message before upgrade: #{inspect(message)}")
    {:noreply, state}
  end

  def handle_info(message, state) do
    case Mint.WebSocket.stream(state.conn, message) do
      :unknown ->
        {:noreply, state}

      {:ok, conn, [{:data, ref, data}]} when ref == state.ref ->
        state = %{state | conn: conn}
        handle_data(data, state)

      {:ok, conn, _responses} ->
        {:noreply, %{state | conn: conn}}

      {:error, conn, reason, _responses} ->
        Logger.warning("[CoreWebSocketWorker] Stream error: #{inspect(reason)}")
        {:stop, {:ws_error, reason}, %{state | conn: conn}}
    end
  end

  @impl GenServer
  def terminate(reason, %{conn: conn} = _state) when not is_nil(conn) do
    Logger.info("[CoreWebSocketWorker] Terminating: #{inspect(reason)}")
    Mint.HTTP.close(conn)
    :ok
  end

  def terminate(reason, _state) do
    Logger.info("[CoreWebSocketWorker] Terminating: #{inspect(reason)}")
    :ok
  end

  # Private

  defp receive_upgrade(conn, ref, deadline \\ nil) do
    deadline = deadline || System.monotonic_time(:millisecond) + 10_000
    remaining = max(0, deadline - System.monotonic_time(:millisecond))

    receive do
      message ->
        case Mint.WebSocket.stream(conn, message) do
          :unknown ->
            receive_upgrade(conn, ref, deadline)

          {:ok, conn, responses} ->
            status = find_response(responses, ref, :status)
            headers = find_response(responses, ref, :headers)
            done = Enum.any?(responses, &match?({:done, ^ref}, &1))

            if status && headers && done do
              Mint.WebSocket.new(conn, ref, status, headers)
            else
              receive_upgrade(conn, ref, deadline)
            end

          {:error, _conn, reason, _responses} ->
            {:error, {:upgrade_failed, reason}}
        end
    after
      remaining -> {:error, :upgrade_timeout}
    end
  end

  defp find_response(responses, ref, :status) do
    case Enum.find(responses, &match?({:status, ^ref, _}, &1)) do
      {:status, _, status} -> status
      nil -> nil
    end
  end

  defp find_response(responses, ref, :headers) do
    case Enum.find(responses, &match?({:headers, ^ref, _}, &1)) do
      {:headers, _, headers} -> headers
      nil -> nil
    end
  end

  defp handle_data(data, state) do
    case Mint.WebSocket.decode(state.websocket, data) do
      {:ok, websocket, frames} ->
        state = %{state | websocket: websocket}
        state = process_frames(frames, state)
        {:noreply, state}

      {:error, websocket, reason} ->
        Logger.warning("[CoreWebSocketWorker] Decode error: #{inspect(reason)}")
        {:noreply, %{state | websocket: websocket}}
    end
  end

  defp process_frames(frames, state) do
    Enum.reduce(frames, state, fn
      {:text, json}, acc ->
        case Jason.decode(json) do
          {:ok, event} ->
            :telemetry.execute(
              [:query_service_ex, :websocket, :message_received],
              %{size_bytes: byte_size(json)},
              %{message_type: "event", event_type: event["event_type"]}
            )

            send(acc.parent, {:websocket_event, event})
            acc

          {:error, reason} ->
            Logger.warning("[CoreWebSocketWorker] JSON parse error: #{inspect(reason)}")
            acc
        end

      {:ping, payload}, acc ->
        send_frame(acc, {:pong, payload})

      {:close, code, reason}, acc ->
        Logger.info("[CoreWebSocketWorker] Server closed: #{code} #{reason}")
        acc

      _frame, acc ->
        acc
    end)
  end

  defp send_frame(state, frame) do
    with {:ok, websocket, data} <- Mint.WebSocket.encode(state.websocket, frame),
         {:ok, conn} <- Mint.WebSocket.stream_request_body(state.conn, state.ref, data) do
      %{state | conn: conn, websocket: websocket}
    else
      _ -> state
    end
  end

  # Resolve hostname, trying IPv6 first for Fly.io .internal DNS compatibility.
  # Returns {address, transport_opts} where address is either an IP tuple or hostname string.
  defp resolve_host(host, timeout) do
    case :inet.getaddr(String.to_charlist(host), :inet6) do
      {:ok, ipv6_addr} ->
        Logger.debug("[CoreWebSocketWorker] Resolved #{host} to IPv6: #{:inet.ntoa(ipv6_addr)}")
        {ipv6_addr, [timeout: timeout]}

      {:error, _} ->
        # Fall back to letting Mint resolve (works for IPv4 / localhost)
        {host, [timeout: timeout]}
    end
  end
end
