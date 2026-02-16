defmodule McpServerElixir.Server do
  @moduledoc """
  Main MCP server process that handles JSON-RPC 2.0 messages over stdio.

  This GenServer reads from stdin and writes to stdout, processing MCP protocol
  messages and routing them to appropriate handlers.
  """

  use GenServer
  require Logger

  alias McpServerElixir.Infrastructure.{ControlPlaneClient, CoreClient}
  alias McpServerElixir.Protocol.{JsonRpc, McpTools}

  @server_info %{
    name: "allsource-mcp-elixir",
    version: "0.1.0"
  }

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, [], name: __MODULE__)
  end

  @impl true
  def init(_opts) do
    # Log to stderr (MCP protocol uses stdout for JSON-RPC)
    :ok = :io.setopts(:standard_error, encoding: :utf8)

    :ok = :io.setopts(:standard_io, encoding: :utf8)

    Logger.info("🌟 AllSource MCP Server (Elixir) starting...")

    Logger.info(
      "📡 Core API: #{Application.get_env(:mcp_server_elixir, :core_url, "http://localhost:3900")}"
    )

    Logger.info(
      "🎛️  Control Plane: #{Application.get_env(:mcp_server_elixir, :control_url, "http://localhost:3901")}"
    )

    # Start reading from stdin using a Task
    {:ok, _pid} = Task.start_link(fn -> stdin_reader_loop() end)

    read_only = Application.get_env(:mcp_server_elixir, :read_only, false)
    control_plane_enabled = System.get_env("ALLSOURCE_CONTROL_URL") != nil

    if read_only, do: Logger.info("🔒 Read-only mode enabled")

    unless control_plane_enabled,
      do:
        Logger.info(
          "ℹ️  Control plane not configured (set ALLSOURCE_CONTROL_URL to enable tenant tools)"
        )

    {:ok,
     %{
       core_client: CoreClient.new(),
       control_client: ControlPlaneClient.new(),
       read_only: read_only,
       control_plane_enabled: control_plane_enabled
     }}
  end

  @impl true
  def handle_info({:stdin_line, line}, state) do
    handle_input(line, state)
    {:noreply, state}
  end

  @impl true
  def handle_info({:stdin_eof}, state) do
    Logger.info("EOF received, shutting down")
    {:stop, :normal, state}
  end

  defp stdin_reader_loop do
    case IO.read(:stdio, :line) do
      :eof ->
        send(__MODULE__, {:stdin_eof})

      {:error, reason} ->
        Logger.error("Error reading stdin: #{inspect(reason)}")
        send(__MODULE__, {:stdin_eof})

      line when is_binary(line) ->
        send(__MODULE__, {:stdin_line, line})
        stdin_reader_loop()
    end
  end

  defp handle_input(line, state) do
    line
    |> String.trim()
    |> case do
      "" ->
        :ok

      json_string ->
        case JsonRpc.parse_request(json_string) do
          {:ok, request} ->
            process_request(request, state)

          {:error, reason} ->
            Logger.warning("Failed to parse JSON-RPC request: #{inspect(reason)}")
            send_error(nil, -32_700, "Parse error", nil)
        end
    end
  end

  defp process_request(%{method: "initialize", params: _params, id: id}, state) do
    response = %{
      jsonrpc: "2.0",
      id: id,
      result: %{
        protocolVersion: "2024-11-05",
        capabilities: %{
          tools: %{}
        },
        serverInfo: @server_info
      }
    }

    send_response(response)
    {:noreply, state}
  end

  defp process_request(%{method: "tools/list", id: id}, state) do
    config = %{
      read_only: Map.get(state, :read_only, false),
      control_plane_enabled: Map.get(state, :control_plane_enabled, false)
    }

    tools = McpTools.list_tools(config)

    response = %{
      jsonrpc: "2.0",
      id: id,
      result: %{
        tools: tools
      }
    }

    send_response(response)
    {:noreply, state}
  end

  defp process_request(%{method: "tools/call", params: params, id: id}, state) do
    tool_name = Map.get(params, "name")
    args = Map.get(params, "arguments", %{})

    case McpTools.call_tool(tool_name, args, state) do
      {:ok, result} ->
        response = %{
          jsonrpc: "2.0",
          id: id,
          result: result
        }

        send_response(response)

      {:error, reason} ->
        send_error(id, -32_603, "Internal error", reason)
    end

    {:noreply, state}
  end

  defp process_request(%{method: "notifications/initialized"}, state) do
    # Handle initialized notification (no response needed)
    {:noreply, state}
  end

  defp process_request(%{method: method, id: id}, state) when is_binary(method) do
    Logger.warning("Unknown method: #{method}")
    send_error(id, -32_601, "Method not found", nil)
    {:noreply, state}
  end

  defp process_request(%{method: method}, state) when is_binary(method) do
    # Unknown notification (no id = no response needed)
    Logger.debug("Ignoring unknown notification: #{method}")
    {:noreply, state}
  end

  defp send_response(response) do
    json = Jason.encode!(response)
    # Write to stdout without newline (MCP protocol expects JSON lines)
    IO.write(:stdio, json <> "\n")
  end

  defp send_error(id, code, message, data) do
    error = %{
      code: code,
      message: message
    }

    error = if data, do: Map.put(error, :data, data), else: error

    response = %{
      jsonrpc: "2.0",
      id: id,
      error: error
    }

    send_response(response)
  end
end
