defmodule QueryServiceEx.Projections.TenantProjections do
  @moduledoc """
  Per-tenant, QS-owned projection state + folding.

  This is the read-model engine for the per-tenant projections feature
  (`docs/proposals/PER_TENANT_PROJECTIONS.md`). It is intentionally SEPARATE from
  the legacy `QueryServiceEx.Application.Services.ProjectionSync`, which syncs
  Core's global engine state and must not be repurposed.

  ## State

  A single GenServer owns two public ETS tables:

    * `:tenant_projection_state` — `{tenant_id, projection, entity_id} -> state`
    * `:tenant_projection_status` — `{tenant_id, projection} -> :building | :ready`

  State is keyed by `{tenant_id, projection, entity_id}` so one tenant's folded
  read-model never mixes with another's. State is a rebuildable read-model; Core
  remains the durable log of record.

  ## Lifecycle

    * `enable/2` — mark status `:building`, spawn a background backfill Task that
      pages the tenant's whole history from Core and folds it through the
      template's reducer, then flips status to `:ready`. Returns immediately.
    * `disable/2` — drop the tenant's state + status for that projection.
    * Live updates — the GenServer subscribes to the tenant-scoped event topic
      `events:<tenant>:all` and folds new events into the affected enabled
      projections incrementally.

  ## Isolation

  Every key carries `tenant_id`; the live subscription is to a tenant-scoped
  PubSub topic only. There is no cross-tenant read or write path.
  """

  use GenServer
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.Projections.Catalog
  alias QueryServiceExWeb.ChannelBroadcaster

  @state_table :tenant_projection_state
  @status_table :tenant_projection_status
  @pubsub QueryServiceEx.PubSub

  # Backfill paging: bounded pages, loop until a short page.
  @backfill_page_size 1_000
  @max_backfill_pages 10_000

  # -- Client API --

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Initialize the ETS tables. Safe to call before the GenServer starts (the
  GenServer also ensures them in `init/1`).
  """
  def init_tables do
    ensure_table(@state_table)
    ensure_table(@status_table)
    :ok
  end

  @doc """
  Enable a template for a tenant: set status `:building` and kick off a
  background backfill. Returns `:ok` immediately, or `{:error, :unknown_template}`.
  """
  @spec enable(String.t(), String.t()) :: :ok | {:error, :unknown_template}
  def enable(tenant_id, template_name)
      when is_binary(tenant_id) and is_binary(template_name) do
    case Catalog.fetch(template_name) do
      {:ok, _template} ->
        GenServer.call(__MODULE__, {:enable, tenant_id, template_name})

      :error ->
        {:error, :unknown_template}
    end
  end

  @doc """
  Disable a projection for a tenant: drop its folded state + status.
  """
  @spec disable(String.t(), String.t()) :: :ok
  def disable(tenant_id, template_name)
      when is_binary(tenant_id) and is_binary(template_name) do
    GenServer.call(__MODULE__, {:disable, tenant_id, template_name})
  end

  @doc """
  Read folded state for `{tenant_id, projection, entity_id}`.

  Returns `{:ok, state}` or `{:error, :not_found}`. Reads straight from ETS.
  """
  @spec get_state(String.t(), String.t(), String.t()) :: {:ok, term()} | {:error, :not_found}
  def get_state(tenant_id, projection, entity_id)
      when is_binary(tenant_id) and is_binary(projection) and is_binary(entity_id) do
    case lookup(@state_table, {tenant_id, projection, entity_id}) do
      {:ok, state} -> {:ok, state}
      :error -> {:error, :not_found}
    end
  end

  @doc """
  Current status of an enabled projection: `:building`, `:ready`, or `nil` if not
  enabled in QS state.
  """
  @spec status(String.t(), String.t()) :: :building | :ready | nil
  def status(tenant_id, projection) when is_binary(tenant_id) and is_binary(projection) do
    case lookup(@status_table, {tenant_id, projection}) do
      {:ok, status} -> status
      :error -> nil
    end
  end

  @doc """
  Summary of a tenant's QS-tracked projections (those with a status entry).

  Returns a list of `%{name: ..., status: "building" | "ready", entity_count: n}`.
  """
  @spec list(String.t()) :: [map()]
  def list(tenant_id) when is_binary(tenant_id) do
    statuses =
      safe_select(@status_table, [
        {{{tenant_id, :"$1"}, :"$2"}, [], [{{:"$1", :"$2"}}]}
      ])

    Enum.map(statuses, fn {name, status} ->
      %{
        name: name,
        status: Atom.to_string(status),
        entity_count: entity_count(tenant_id, name)
      }
    end)
  end

  # -- Server callbacks --

  @impl GenServer
  def init(_opts) do
    init_tables()

    # Live updates fold into all tenants' enabled projections. We subscribe to a
    # per-tenant topic lazily the first time a tenant enables anything (a tenant
    # may not exist yet at boot). Track which tenant topics we are subscribed to.
    {:ok, %{subscribed_tenants: MapSet.new()}}
  end

  @impl GenServer
  def handle_call({:enable, tenant_id, template_name}, _from, state) do
    {:ok, template} = Catalog.fetch(template_name)

    :ets.insert(@status_table, {{tenant_id, template_name}, :building})
    state = ensure_tenant_subscription(state, tenant_id)

    server = self()

    Task.Supervisor.start_child(QueryServiceEx.Projections.BackfillSupervisor, fn ->
      backfill(server, tenant_id, template)
    end)

    {:reply, :ok, state}
  end

  @impl GenServer
  def handle_call({:disable, tenant_id, template_name}, _from, state) do
    # Drop the status, then all per-entity state rows for this projection.
    :ets.delete(@status_table, {tenant_id, template_name})

    safe_match_delete(@state_table, {{tenant_id, template_name, :_}, :_})

    {:reply, :ok, state}
  end

  # Backfill completion: flip status to :ready (only if still enabled).
  @impl GenServer
  def handle_cast({:backfill_done, tenant_id, projection}, state) do
    case lookup(@status_table, {tenant_id, projection}) do
      {:ok, _} ->
        :ets.insert(@status_table, {{tenant_id, projection}, :ready})

        Logger.info("[TenantProjections] backfill ready",
          tenant_id: tenant_id,
          projection: projection
        )

      :error ->
        # Disabled mid-backfill; do not resurrect status.
        :ok
    end

    {:noreply, state}
  end

  # Live event from the tenant-scoped "all" topic — fold into each enabled
  # projection for that tenant.
  @impl GenServer
  def handle_info({:new_event, event}, state) when is_map(event) do
    case event["tenant_id"] || event[:tenant_id] do
      tenant_id when is_binary(tenant_id) ->
        fold_live_event(tenant_id, event)
        {:noreply, state}

      _ ->
        {:noreply, state}
    end
  end

  @impl GenServer
  def handle_info(_msg, state), do: {:noreply, state}

  # -- Internal: subscriptions --

  defp ensure_tenant_subscription(state, tenant_id) do
    if MapSet.member?(state.subscribed_tenants, tenant_id) do
      state
    else
      # Tenant-scoped topic ONLY (events:<tenant>:all). Never a global topic —
      # the tenant-isolation gate forbids it and it would leak cross-tenant.
      Phoenix.PubSub.subscribe(@pubsub, "events:#{tenant_id}:all")
      %{state | subscribed_tenants: MapSet.put(state.subscribed_tenants, tenant_id)}
    end
  end

  # -- Internal: live folding --

  defp fold_live_event(tenant_id, event) do
    @status_table
    |> safe_select([{{{tenant_id, :"$1"}, :"$2"}, [], [{{:"$1", :"$2"}}]}])
    |> Enum.each(fn {projection, _status} ->
      case Catalog.fetch(projection) do
        {:ok, template} -> apply_event(tenant_id, projection, template, event)
        :error -> :ok
      end
    end)
  end

  defp apply_event(tenant_id, projection, template, event) do
    entity_id = template.entity_key.(event)
    key = {tenant_id, projection, entity_id}

    current =
      case lookup(@state_table, key) do
        {:ok, s} -> s
        :error -> template.initial
      end

    new_state = template.reduce.(current, event)
    :ets.insert(@state_table, {key, new_state})

    broadcast_state(tenant_id, projection, entity_id, new_state)
    new_state
  end

  defp broadcast_state(tenant_id, projection, entity_id, state) do
    ChannelBroadcaster.broadcast_projection_update(tenant_id, projection, entity_id, state)
  rescue
    _ -> :ok
  end

  # -- Internal: backfill --

  defp backfill(server, tenant_id, template) do
    projection = template.name

    Logger.info("[TenantProjections] backfill start",
      tenant_id: tenant_id,
      projection: projection
    )

    fold_history(tenant_id, template, 0, 0)
    GenServer.cast(server, {:backfill_done, tenant_id, projection})
  rescue
    error ->
      Logger.error(
        "[TenantProjections] backfill crashed: #{inspect(error)}",
        tenant_id: tenant_id,
        projection: template.name
      )

      # Leave status :building so a re-enable / refresh can retry; do not flip
      # to ready on a partial fold.
      :ok
  end

  defp fold_history(_tenant_id, _template, offset, page_no)
       when page_no >= @max_backfill_pages do
    Logger.warning("[TenantProjections] backfill hit max pages at offset #{offset}")
    :ok
  end

  defp fold_history(tenant_id, template, offset, page_no) do
    params = %{limit: @backfill_page_size, offset: offset, order: "asc"}

    case query_events(tenant_id, params) do
      {:ok, events} when is_list(events) ->
        Enum.each(events, fn event ->
          fold_into_state(tenant_id, template, event)
        end)

        if length(events) < @backfill_page_size do
          :ok
        else
          fold_history(tenant_id, template, offset + length(events), page_no + 1)
        end

      {:error, reason} ->
        Logger.warning(
          "[TenantProjections] backfill page failed at offset #{offset}: #{inspect(reason)}",
          tenant_id: tenant_id,
          projection: template.name
        )

        :ok
    end
  end

  # Backfill fold writes directly to ETS (no live broadcast per historical event).
  defp fold_into_state(tenant_id, template, event) do
    entity_id = template.entity_key.(event)
    key = {tenant_id, template.name, entity_id}

    current =
      case lookup(@state_table, key) do
        {:ok, s} -> s
        :error -> template.initial
      end

    :ets.insert(@state_table, {key, template.reduce.(current, event)})
  end

  # The backfill source. Defaults to Core's tenant-scoped event query; tests can
  # override with `config :query_service_ex, :tenant_projection_query_fun, fun/2`.
  defp query_events(tenant_id, params) do
    case Application.get_env(:query_service_ex, :tenant_projection_query_fun) do
      fun when is_function(fun, 2) -> fun.(tenant_id, params)
      _ -> RustCoreClient.query_events(tenant_id, params)
    end
  end

  # -- Internal: ETS helpers --

  defp ensure_table(table) do
    if :ets.whereis(table) == :undefined do
      :ets.new(table, [:set, :public, :named_table, read_concurrency: true])
    end
  end

  defp lookup(table, key) do
    case :ets.lookup(table, key) do
      [{^key, value}] -> {:ok, value}
      [] -> :error
    end
  rescue
    ArgumentError -> :error
  end

  defp safe_select(table, spec) do
    :ets.select(table, spec)
  rescue
    ArgumentError -> []
  end

  defp safe_match_delete(table, pattern) do
    :ets.match_delete(table, pattern)
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp entity_count(tenant_id, projection) do
    safe_select(@state_table, [
      {{{tenant_id, projection, :"$1"}, :_}, [], [true]}
    ])
    |> length()
  end
end
