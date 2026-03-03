import Config

core_mode = System.get_env("CORE_MODE", "remote")

case core_mode do
  "embedded" ->
    config :mcp_server_elixir,
      core_mode: :embedded,
      core_backend: McpServerElixir.Infrastructure.CoreEmbedded,
      core_ws_enabled: false,
      embedded_config: %{
        data_dir: System.get_env("CORE_DATA_DIR", ""),
        node_id: String.to_integer(System.get_env("CORE_NODE_ID", "1")),
        wal_fsync_interval_ms: String.to_integer(System.get_env("CORE_WAL_FSYNC_MS", "100")),
        parquet_flush_interval_secs:
          String.to_integer(System.get_env("CORE_PARQUET_FLUSH_SECS", "300"))
      }

  _ ->
    config :mcp_server_elixir,
      core_mode: :remote,
      core_backend: McpServerElixir.Infrastructure.CoreClient
end
