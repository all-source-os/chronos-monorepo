defmodule McpServerElixir.Application do
  @moduledoc """
  MCP Server Application - Main entry point for the Model Context Protocol server.

  This server communicates via stdio (standard input/output) using JSON-RPC 2.0
  to enable AI assistants like Claude Desktop to interact with AllSource's
  temporal event store.

  Supports two backend modes via CORE_MODE env var:
  - `remote` (default): Connects to a remote Core instance via HTTP/WS
  - `embedded`: Runs Core in-process via Rustler NIF
  """

  use Application

  @impl true
  def start(_type, _args) do
    core_mode = Application.get_env(:mcp_server_elixir, :core_mode, :remote)

    children =
      [
        # PubSub for local event broadcasting
        {Phoenix.PubSub, name: McpServerElixir.PubSub}
      ] ++
        mode_children(core_mode) ++
        [
          # Conversation context manager for multi-turn queries
          {McpServerElixir.Context.ConversationContext, []},

          # Start the MCP server process
          {McpServerElixir.Server, []}
        ]

    opts = [strategy: :one_for_one, name: McpServerElixir.Supervisor]
    Supervisor.start_link(children, opts)
  end

  defp mode_children(:embedded) do
    [
      {McpServerElixir.Infrastructure.CoreEmbedded.Supervisor, []},
      {McpServerElixir.Infrastructure.CoreEmbedded.SyncWorker, []}
    ]
  end

  defp mode_children(:remote) do
    [{McpServerElixir.Infrastructure.CoreWebSocketClient, []}]
  end
end
