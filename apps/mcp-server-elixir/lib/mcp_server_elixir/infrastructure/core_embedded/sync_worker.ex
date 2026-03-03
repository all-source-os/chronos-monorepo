if System.get_env("CORE_MODE") == "embedded" do
  defmodule McpServerElixir.Infrastructure.CoreEmbedded.SyncWorker do
    @moduledoc """
    Periodic sync worker for embedded Core mode.

    When CORE_SYNC_URL is configured, syncs local embedded Core data to a remote
    Core instance on a timer. If CORE_SYNC_URL is not set, the worker is a no-op.
    """

    use GenServer

    require Logger

    alias McpServerElixir.Infrastructure.CoreNif

    def start_link(opts \\ []) do
      GenServer.start_link(__MODULE__, opts, name: __MODULE__)
    end

    @impl true
    def init(_opts) do
      sync_url = System.get_env("CORE_SYNC_URL")
      interval = String.to_integer(System.get_env("CORE_SYNC_INTERVAL_MS", "60000"))
      node_id = System.get_env("CORE_NODE_ID", "1")

      if sync_url do
        Logger.info("[SyncWorker] Starting with url=#{sync_url} interval=#{interval}ms")
        schedule_sync(interval)
        {:ok, %{sync_url: sync_url, interval: interval, node_id: node_id}}
      else
        Logger.info("[SyncWorker] CORE_SYNC_URL not set, sync disabled")
        {:ok, %{sync_url: nil, interval: interval, node_id: node_id}}
      end
    end

    @impl true
    def handle_info(:sync, %{sync_url: nil} = state) do
      {:noreply, state}
    end

    def handle_info(:sync, %{sync_url: sync_url, interval: interval, node_id: node_id} = state) do
      resource = McpServerElixir.Infrastructure.CoreEmbedded.Supervisor.resource()

      case CoreNif.nif_sync(resource, sync_url, node_id) do
        {:ok, stats} ->
          pushed = Map.get(stats, :pushed, Map.get(stats, "pushed", 0))
          pulled = Map.get(stats, :pulled, Map.get(stats, "pulled", 0))
          conflicts = Map.get(stats, :conflicts, Map.get(stats, "conflicts", 0))

          Logger.info(
            "[SyncWorker] Synced: pushed=#{pushed}, pulled=#{pulled}, conflicts=#{conflicts}"
          )

        {:error, reason} ->
          Logger.warning("[SyncWorker] Sync failed: #{inspect(reason)}")
      end

      schedule_sync(interval)
      {:noreply, state}
    end

    defp schedule_sync(interval) do
      Process.send_after(self(), :sync, interval)
    end
  end
end

# if CORE_MODE == "embedded"
