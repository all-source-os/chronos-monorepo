defmodule QueryServiceExWeb.WebhookSubscriptionController do
  @moduledoc """
  Controller for managing outbound webhook subscriptions.

  Allows tenants to register webhook URLs that will receive event deliveries
  via HTTP POST when matching events are ingested.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  action_fallback(QueryServiceExWeb.FallbackController)

  require Logger

  @doc """
  List webhook subscriptions for the current tenant.

  GET /api/webhooks
  """
  def index(conn, _params) do
    tenant_id = get_tenant_id!(conn)

    case RustCoreClient.list_webhooks(tenant_id) do
      {:ok, %{webhooks: webhooks, total: total}} ->
        json(conn, %{data: webhooks, count: length(webhooks), total: total})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Register a new webhook subscription.

  POST /api/webhooks

  Body:
    - url: The URL to deliver events to (required)
    - event_types: List of event type patterns to filter (optional, e.g. ["user.*", "order.created"])
    - entity_ids: List of entity IDs to filter (optional)
    - secret: Signing secret for payload verification (optional, auto-generated if not provided)
    - description: Human-readable description (optional)
  """
  def create(conn, params) do
    tenant_id = get_tenant_id!(conn)

    webhook_params = %{
      url: params["url"],
      event_types: params["event_types"] || [],
      entity_ids: params["entity_ids"] || [],
      secret: params["secret"],
      description: params["description"]
    }

    case RustCoreClient.register_webhook(tenant_id, webhook_params) do
      {:ok, webhook} ->
        conn
        |> put_status(:created)
        |> json(%{data: webhook})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get a specific webhook subscription.

  GET /api/webhooks/:id
  """
  def show(conn, %{"id" => webhook_id}) do
    case RustCoreClient.get_webhook(webhook_id) do
      {:ok, webhook} ->
        # Verify tenant owns this webhook
        tenant_id = get_tenant_id!(conn)

        if webhook["tenant_id"] == tenant_id do
          json(conn, %{data: webhook})
        else
          conn
          |> put_status(:not_found)
          |> json(%{error: "Webhook not found"})
        end

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Webhook not found"})
    end
  end

  @doc """
  Update a webhook subscription.

  PUT /api/webhooks/:id

  Body (all optional):
    - url: Updated URL
    - event_types: Updated event type filters
    - entity_ids: Updated entity ID filters
    - active: Enable/disable the webhook
    - description: Updated description
  """
  def update(conn, %{"id" => webhook_id} = params) do
    update_params =
      %{}
      |> maybe_put(:url, params["url"])
      |> maybe_put(:event_types, params["event_types"])
      |> maybe_put(:entity_ids, params["entity_ids"])
      |> maybe_put(:active, params["active"])
      |> maybe_put(:description, params["description"])

    case RustCoreClient.update_webhook(webhook_id, update_params) do
      {:ok, webhook} ->
        json(conn, %{data: webhook})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Webhook not found"})
    end
  end

  @doc """
  Delete a webhook subscription.

  DELETE /api/webhooks/:id
  """
  def delete(conn, %{"id" => webhook_id}) do
    case RustCoreClient.delete_webhook(webhook_id) do
      {:ok, _body} ->
        conn
        |> put_status(:ok)
        |> json(%{deleted: true})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Webhook not found"})
    end
  end

  @doc """
  List delivery history for a webhook.

  GET /api/webhooks/:id/deliveries
  """
  def deliveries(conn, %{"id" => webhook_id} = params) do
    limit = parse_int(params["limit"], 50)

    case RustCoreClient.list_webhook_deliveries(webhook_id, limit: limit) do
      {:ok, body} ->
        json(conn, %{data: body})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Webhook not found"})
    end
  end

  # -------------------------------------------------------------------
  # Helpers
  # -------------------------------------------------------------------

  defp get_tenant_id!(conn) do
    conn.assigns[:tenant_id] || conn.private[:allsource_tenant_id] ||
      raise "No tenant_id found in connection"
  end

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)

  defp parse_int(nil, default), do: default

  defp parse_int(val, default) when is_binary(val) do
    case Integer.parse(val) do
      {int, _} -> int
      :error -> default
    end
  end

  defp parse_int(val, _default) when is_integer(val), do: val
end
