defmodule McpServerElixir.Application do
  @moduledoc """
  MCP Server Application - Main entry point for the Model Context Protocol server.

  This server communicates via stdio (standard input/output) using JSON-RPC 2.0
  to enable AI assistants like Claude Desktop to interact with AllSource's
  temporal event store.
  """

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # PubSub for local event broadcasting from WebSocket
      {Phoenix.PubSub, name: McpServerElixir.PubSub},

      # WebSocket client for real-time events from Core
      {McpServerElixir.Infrastructure.CoreWebSocketClient, []},

      # Start the MCP server process
      {McpServerElixir.Server, []}
    ]

    opts = [strategy: :one_for_one, name: McpServerElixir.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
