defmodule QueryServiceEx.DemoReset do
  @moduledoc """
  Scheduled GenServer that resets demo tenant data daily.

  Deletes all demo tenants from Core and re-seeds fresh sample data.
  Reset hour is configurable via DEMO_RESET_HOUR env var (default: 4 = 4 AM UTC).

  Only runs when DEMO_RESET_ENABLED=true is set. Disabled by default.
  """

  use GenServer

  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @check_interval :timer.minutes(15)

  # Client API

  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc "Force an immediate reset cycle. Used for testing."
  def reset_now do
    GenServer.call(__MODULE__, :reset_now, 60_000)
  end

  # Server callbacks

  @impl true
  def init(_opts) do
    if enabled?() do
      Logger.info(
        "[DemoReset] Started. Reset hour: #{reset_hour()} UTC, check interval: #{div(@check_interval, 60_000)}m"
      )

      schedule_check()
      {:ok, %{last_reset_date: nil}}
    else
      Logger.info("[DemoReset] Disabled (set DEMO_RESET_ENABLED=true to enable)")
      :ignore
    end
  end

  @impl true
  def handle_info(:check, state) do
    today = Date.utc_today()
    now = DateTime.utc_now()

    state =
      if now.hour >= reset_hour() and state.last_reset_date != today do
        perform_reset()
        %{state | last_reset_date: today}
      else
        state
      end

    schedule_check()
    {:noreply, state}
  end

  @impl true
  def handle_call(:reset_now, _from, state) do
    result = perform_reset()
    {:reply, result, %{state | last_reset_date: Date.utc_today()}}
  end

  # Private

  defp perform_reset do
    Logger.info("[DemoReset] Starting daily demo data reset")

    # Query Core for demo tenants (list tenants, filter by is_demo)
    case list_demo_tenants() do
      {:ok, tenants} ->
        Logger.info("[DemoReset] Found #{length(tenants)} demo tenant(s) to reset")

        for tenant <- tenants do
          tenant_id = tenant["id"]
          Logger.info("[DemoReset] Resetting tenant #{tenant_id}")

          case RustCoreClient.delete_tenant(tenant_id) do
            :ok ->
              Logger.info("[DemoReset] Deleted tenant #{tenant_id}")

            {:error, reason} ->
              Logger.warning(
                "[DemoReset] Failed to delete tenant #{tenant_id}: #{inspect(reason)}"
              )
          end
        end

        # Re-seed demo data (idempotent)
        case RustCoreClient.demo_seed() do
          {:ok, _} -> Logger.info("[DemoReset] Demo data re-seeded successfully")
          {:error, reason} -> Logger.warning("[DemoReset] Failed to re-seed: #{inspect(reason)}")
        end

        :ok

      {:error, reason} ->
        Logger.warning("[DemoReset] Failed to list demo tenants: #{inspect(reason)}")
        {:error, reason}
    end
  end

  defp list_demo_tenants do
    case Tesla.get(RustCoreClient.read_client(), "/api/v1/tenants") do
      {:ok, %Tesla.Env{status: 200, body: %{"tenants" => tenants}}} ->
        demo_tenants = Enum.filter(tenants, fn t -> t["is_demo"] == true end)
        {:ok, demo_tenants}

      {:ok, %Tesla.Env{status: 200, body: tenants}} when is_list(tenants) ->
        demo_tenants = Enum.filter(tenants, fn t -> t["is_demo"] == true end)
        {:ok, demo_tenants}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp schedule_check do
    Process.send_after(self(), :check, @check_interval)
  end

  defp enabled? do
    System.get_env("DEMO_RESET_ENABLED") in ["true", "1"]
  end

  defp reset_hour do
    case System.get_env("DEMO_RESET_HOUR") do
      nil -> 4
      val -> String.to_integer(val)
    end
  end
end
