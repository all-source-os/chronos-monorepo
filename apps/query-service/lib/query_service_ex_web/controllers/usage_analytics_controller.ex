defmodule QueryServiceExWeb.UsageAnalyticsController do
  @moduledoc """
  Controller for customer-facing usage analytics.

  Provides a breakdown of event store usage including event type distribution,
  top entity IDs, and ingestion rate over time. Supports time range filtering.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.TenantCache

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  @valid_ranges ~w(24h 7d 30d 90d)

  @doc """
  Returns usage analytics breakdown for the authenticated tenant.

  GET /api/tenants/me/analytics
  Query params:
    - range: time range filter (24h, 7d, 30d, 90d). Default: 7d
  """
  def show(conn, params) do
    with {:ok, tenant} <- get_tenant(conn) do
      tenant_id = tenant["id"]
      range = validate_range(params["range"])
      since = range_to_since(range)

      tasks = [
        Task.async(fn -> fetch_event_type_distribution(tenant_id, since) end),
        Task.async(fn -> fetch_top_entities(tenant_id, since) end),
        Task.async(fn -> fetch_ingestion_rate(tenant_id, since, range) end)
      ]

      [type_dist, top_entities, ingestion_rate] =
        Task.await_many(tasks, 15_000)

      conn
      |> put_status(:ok)
      |> json(%{
        data: %{
          range: range,
          since: DateTime.to_iso8601(since),
          event_type_distribution: type_dist,
          top_entity_ids: top_entities,
          ingestion_rate: ingestion_rate
        }
      })
    end
  end

  defp get_tenant(conn) do
    if DevMode.auth_disabled?() do
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()
      {:ok, tenant}
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user[:tenant_id]

      TenantCache.fetch(tenant_id, fn ->
        case RustCoreClient.get_tenant(tenant_id) do
          {:ok, tenant} -> tenant
          {:error, _} -> nil
        end
      end)
      |> case do
        nil -> {:error, :not_found}
        tenant -> {:ok, tenant}
      end
    end
  end

  defp validate_range(range) when range in @valid_ranges, do: range
  defp validate_range(_), do: "7d"

  defp range_to_since(range) do
    seconds =
      case range do
        "24h" -> 86_400
        "7d" -> 7 * 86_400
        "30d" -> 30 * 86_400
        "90d" -> 90 * 86_400
      end

    DateTime.utc_now() |> DateTime.add(-seconds, :second)
  end

  defp fetch_event_type_distribution(tenant_id, since) do
    query = %{
      tenant_id: tenant_id,
      since: DateTime.to_iso8601(since),
      limit: 10_000
    }

    case RustCoreClient.query_events(tenant_id, query) do
      {:ok, events} when is_list(events) ->
        events
        |> Enum.frequencies_by(fn e -> e["event_type"] || "unknown" end)
        |> Enum.map(fn {type, count} -> %{event_type: type, count: count} end)
        |> Enum.sort_by(& &1.count, :desc)
        |> Enum.take(20)

      _ ->
        []
    end
  rescue
    _ -> []
  end

  defp fetch_top_entities(tenant_id, since) do
    query = %{
      tenant_id: tenant_id,
      since: DateTime.to_iso8601(since),
      limit: 10_000
    }

    case RustCoreClient.query_events(tenant_id, query) do
      {:ok, events} when is_list(events) ->
        events
        |> Enum.frequencies_by(fn e -> e["entity_id"] || "unknown" end)
        |> Enum.map(fn {entity_id, count} ->
          %{entity_id: entity_id, event_count: count}
        end)
        |> Enum.sort_by(& &1.event_count, :desc)
        |> Enum.take(10)

      _ ->
        []
    end
  rescue
    _ -> []
  end

  defp fetch_ingestion_rate(tenant_id, since, range) do
    window = ingestion_window(range)

    query = %{
      tenant_id: tenant_id,
      since: DateTime.to_iso8601(since),
      limit: 10_000
    }

    case RustCoreClient.query_events(tenant_id, query) do
      {:ok, events} when is_list(events) ->
        bucket_events(events, window)

      _ ->
        []
    end
  rescue
    _ -> []
  end

  defp ingestion_window("24h"), do: :hour
  defp ingestion_window("7d"), do: :day
  defp ingestion_window("30d"), do: :day
  defp ingestion_window("90d"), do: :week

  defp bucket_events(events, window) do
    events
    |> Enum.group_by(fn event ->
      case DateTime.from_iso8601(event["timestamp"] || "") do
        {:ok, dt, _} -> truncate_to(dt, window)
        _ -> nil
      end
    end)
    |> Map.delete(nil)
    |> Enum.map(fn {bucket, evts} ->
      %{timestamp: DateTime.to_iso8601(bucket), count: length(evts)}
    end)
    |> Enum.sort_by(& &1.timestamp)
  end

  defp truncate_to(dt, :hour) do
    %{dt | minute: 0, second: 0, microsecond: {0, 0}}
  end

  defp truncate_to(dt, :day) do
    %{dt | hour: 0, minute: 0, second: 0, microsecond: {0, 0}}
  end

  defp truncate_to(dt, :week) do
    day_of_week = Date.day_of_week(DateTime.to_date(dt))
    days_since_monday = day_of_week - 1

    dt
    |> DateTime.add(-days_since_monday * 86_400, :second)
    |> then(fn d ->
      %{d | hour: 0, minute: 0, second: 0, microsecond: {0, 0}}
    end)
  end
end
