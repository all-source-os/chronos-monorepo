defmodule QueryServiceExWeb.ReplayController do
  @moduledoc """
  Tenant-scoped projection rebuild operations.

  Replays fold one enabled Query Service projection from the authenticated
  tenant's immutable event history. The current read-model stays live until a
  successful atomic replacement. Core's global projection replay API is not
  exposed here because it cannot provide tenant-safe projection ownership.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Projections.Enablement
  alias QueryServiceEx.Projections.TenantProjections

  action_fallback(QueryServiceExWeb.FallbackController)

  def index(conn, _params) do
    tenant_id = get_tenant_id!(conn)
    replays = TenantProjections.list_replays(tenant_id)
    json(conn, %{data: replays, count: length(replays), total: length(replays)})
  end

  def create(conn, params) do
    tenant_id = get_tenant_id!(conn)
    projection_name = params["projection_name"]

    with true <- is_binary(projection_name) and projection_name != "",
         {:ok, enabled} <- Enablement.enabled_set(tenant_id),
         true <- projection_name in enabled,
         {:ok, replay} <- TenantProjections.rebuild(tenant_id, projection_name) do
      conn
      |> put_status(:accepted)
      |> json(%{data: replay})
    else
      false ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: "Choose an enabled projection to rebuild"})

      {:error, :already_running} ->
        conn
        |> put_status(:conflict)
        |> json(%{error: "This projection already has a replay running"})

      {:error, :projection_not_enabled} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: "Projection is not enabled for this tenant"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string_reason(reason)})
    end
  end

  def show(conn, %{"id" => replay_id}) do
    tenant_id = get_tenant_id!(conn)

    case TenantProjections.get_replay(tenant_id, replay_id) do
      {:ok, replay} ->
        json(conn, %{data: replay})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  def cancel(conn, %{"id" => replay_id}) do
    tenant_id = get_tenant_id!(conn)

    case TenantProjections.cancel_replay(tenant_id, replay_id) do
      {:ok, replay} ->
        json(conn, %{data: replay})

      {:error, :not_running} ->
        conn
        |> put_status(:conflict)
        |> json(%{error: "Replay is no longer running"})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  def delete(conn, %{"id" => replay_id}) do
    tenant_id = get_tenant_id!(conn)

    case TenantProjections.delete_replay(tenant_id, replay_id) do
      :ok ->
        json(conn, %{deleted: true})

      {:error, :still_running} ->
        conn
        |> put_status(:conflict)
        |> json(%{error: "Cancel replay before removing it"})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      tenant_id when is_binary(tenant_id) -> tenant_id
      _ -> raise "Tenant context required but not present - security violation"
    end
  end

  defp to_string_reason(reason) when is_binary(reason), do: reason
  defp to_string_reason(reason), do: inspect(reason)
end
