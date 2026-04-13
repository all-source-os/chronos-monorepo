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
    # The CoreEmbedded modules are wrapped in a compile-time
    # `if System.get_env("CORE_MODE") == "embedded"` guard in
    # lib/mcp_server_elixir/infrastructure/core_{embedded,nif}.ex so that a
    # standard (remote-mode) release doesn't require the Rust toolchain to
    # build the Rustler NIF. If the release was compiled without
    # `CORE_MODE=embedded` set, these modules don't exist at runtime and the
    # error the user sees is a cryptic "supervisor does not exist" crash.
    # Detect that case explicitly and fail loudly with an actionable message.
    if Code.ensure_loaded?(McpServerElixir.Infrastructure.CoreEmbedded.Supervisor) do
      [
        {McpServerElixir.Infrastructure.CoreEmbedded.Supervisor, []},
        {McpServerElixir.Infrastructure.CoreEmbedded.SyncWorker, []}
      ]
    else
      IO.puts(:stderr, """

      ❌ CORE_MODE=embedded was requested but this release does not include
         the embedded Core modules (McpServerElixir.Infrastructure.CoreEmbedded).

         The embedded modules are only compiled when the release is built
         with `CORE_MODE=embedded` in the build environment (so that the
         Rustler NIF for `allsource-core` is built). The default published
         image is compiled for `CORE_MODE=remote`.

         Options:
           1. Use CORE_MODE=remote and point at a running Core HTTP endpoint.
           2. Rebuild the image with the Rust toolchain installed and
              `CORE_MODE=embedded` set during `mix compile`.

         See GitHub issue #129 for the full build-system fix.
      """)

      System.halt(1)
    end
  end

  defp mode_children(:remote) do
    [{McpServerElixir.Infrastructure.CoreWebSocketClient, []}]
  end
end
