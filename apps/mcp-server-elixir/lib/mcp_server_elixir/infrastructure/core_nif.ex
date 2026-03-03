# Only compile when CORE_MODE=embedded (requires Rust toolchain + NIF)
if System.get_env("CORE_MODE") == "embedded" do
  defmodule McpServerElixir.Infrastructure.CoreNif do
    @moduledoc """
    Rustler NIF bridge to the embedded AllSource Core.

    Provides low-level NIF functions that wrap EmbeddedCore's API.
    The `CoreEmbedded` module delegates to these functions to implement
    the `CoreBackend` behaviour.

    This module is only compiled when CORE_MODE=embedded. In HTTP/Docker mode,
    this module and all embedded modules are excluded from compilation.
    """

    use Rustler, otp_app: :mcp_server_elixir, crate: "core_nif"

    # Lifecycle
    def nif_ping, do: :erlang.nif_error(:not_loaded)
    def nif_open(_config), do: :erlang.nif_error(:not_loaded)
    def nif_shutdown(_resource), do: :erlang.nif_error(:not_loaded)

    # Core operations
    def nif_query(_resource, _params), do: :erlang.nif_error(:not_loaded)
    def nif_ingest(_resource, _params), do: :erlang.nif_error(:not_loaded)
    def nif_get_stats(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_get_snapshot(_resource, _entity_id), do: :erlang.nif_error(:not_loaded)
    def nif_reconstruct_state(_resource, _entity_id, _opts), do: :erlang.nif_error(:not_loaded)
    def nif_query_toon(_resource, _params), do: :erlang.nif_error(:not_loaded)

    # Search (stubs in embedded mode)
    def nif_semantic_search(_resource, _params), do: :erlang.nif_error(:not_loaded)
    def nif_hybrid_search(_resource, _params), do: :erlang.nif_error(:not_loaded)

    # Schema (stubs in embedded mode)
    def nif_list_schemas(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_register_schema(_resource, _params), do: :erlang.nif_error(:not_loaded)
    def nif_validate_schema(_resource, _params), do: :erlang.nif_error(:not_loaded)
    def nif_get_schema(_resource, _event_type), do: :erlang.nif_error(:not_loaded)

    # Operational
    def nif_compact_storage(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_storage_stats(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_wal_status(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_health_deep(_resource), do: :erlang.nif_error(:not_loaded)
    def nif_durability_status(_resource), do: :erlang.nif_error(:not_loaded)

    # Analytics (stubs in embedded mode)
    def nif_analytics_frequency(_resource, _opts), do: :erlang.nif_error(:not_loaded)
    def nif_analytics_summary(_resource, _opts), do: :erlang.nif_error(:not_loaded)
    def nif_analytics_correlation(_resource, _opts), do: :erlang.nif_error(:not_loaded)

    # Sync
    def nif_sync(_resource, _remote_url, _node_id), do: :erlang.nif_error(:not_loaded)
  end
end
