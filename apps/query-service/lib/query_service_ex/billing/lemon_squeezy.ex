defmodule QueryServiceEx.Billing.LemonSqueezy do
  @moduledoc """
  LemonSqueezy API client for billing operations.

  Provides functions for:
  - Creating checkout sessions
  - Managing subscriptions
  - Generating customer portal URLs
  - Usage-based billing (metered billing)

  Uses Tesla HTTP client with JSON middleware.
  """

  require Logger

  @base_url "https://api.lemonsqueezy.com/v1"

  # -------------------------------------------------------------------
  # Checkout
  # -------------------------------------------------------------------

  @doc """
  Creates a checkout session for a new subscription.

  Returns the checkout URL where the user should be redirected.
  """
  def create_checkout(params) do
    store_id = get_config(:store_id)

    payload = %{
      "data" => %{
        "type" => "checkouts",
        "attributes" => %{
          "checkout_data" => %{
            "email" => params.email,
            "name" => params.name,
            "custom" => %{
              "tenant_id" => params.tenant_id
            }
          },
          "product_options" => %{
            "redirect_url" => get_redirect_url()
          }
        },
        "relationships" => %{
          "store" => %{
            "data" => %{
              "type" => "stores",
              "id" => store_id
            }
          },
          "variant" => %{
            "data" => %{
              "type" => "variants",
              "id" => params.variant_id
            }
          }
        }
      }
    }

    case http_post("/checkouts", payload) do
      {:ok, %{"data" => %{"attributes" => %{"url" => url}}}} ->
        {:ok, url}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # -------------------------------------------------------------------
  # Customer Portal
  # -------------------------------------------------------------------

  @doc """
  Gets the customer portal URL for managing subscriptions.
  """
  def get_customer_portal_url(customer_id) do
    case http_get("/customers/#{customer_id}") do
      {:ok, %{"data" => %{"attributes" => %{"urls" => %{"customer_portal" => url}}}}} ->
        {:ok, url}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # -------------------------------------------------------------------
  # Subscription Management
  # -------------------------------------------------------------------

  @doc """
  Gets subscription details.
  """
  def get_subscription(subscription_id) do
    case http_get("/subscriptions/#{subscription_id}") do
      {:ok, %{"data" => data}} ->
        {:ok, data}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Cancels a subscription.
  """
  def cancel_subscription(subscription_id) do
    case http_delete("/subscriptions/#{subscription_id}") do
      {:ok, _} -> :ok
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Resumes a cancelled subscription.
  """
  def resume_subscription(subscription_id) do
    payload = %{
      "data" => %{
        "type" => "subscriptions",
        "id" => subscription_id,
        "attributes" => %{
          "cancelled" => false
        }
      }
    }

    case http_patch("/subscriptions/#{subscription_id}", payload) do
      {:ok, _} -> :ok
      {:error, reason} -> {:error, reason}
    end
  end

  # -------------------------------------------------------------------
  # Usage-Based Billing
  # -------------------------------------------------------------------

  @doc """
  Reports usage for metered billing.

  This is used for hybrid pricing where base subscription + usage overage.
  """
  def report_usage(subscription_item_id, quantity, action \\ "increment") do
    payload = %{
      "data" => %{
        "type" => "usage-records",
        "attributes" => %{
          "quantity" => quantity,
          "action" => action
        },
        "relationships" => %{
          "subscription_item" => %{
            "data" => %{
              "type" => "subscription-items",
              "id" => subscription_item_id
            }
          }
        }
      }
    }

    case http_post("/usage-records", payload) do
      {:ok, %{"data" => data}} ->
        {:ok, data}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # -------------------------------------------------------------------
  # HTTP Client using Tesla
  # -------------------------------------------------------------------

  defp client do
    api_key = get_config(:api_key)

    if is_nil(api_key) do
      Logger.warning("[LemonSqueezy] API key not configured")
      nil
    else
      middleware = [
        {Tesla.Middleware.BaseUrl, @base_url},
        Tesla.Middleware.JSON,
        {Tesla.Middleware.Headers,
         [
           {"Authorization", "Bearer #{api_key}"},
           {"Accept", "application/vnd.api+json"},
           {"Content-Type", "application/vnd.api+json"}
         ]},
        {Tesla.Middleware.Timeout, timeout: 30_000}
      ]

      Tesla.client(middleware)
    end
  end

  defp http_get(path) do
    case client() do
      nil -> {:error, :not_configured}
      client -> Tesla.get(client, path) |> handle_response()
    end
  end

  defp http_post(path, body) do
    case client() do
      nil -> {:error, :not_configured}
      client -> Tesla.post(client, path, body) |> handle_response()
    end
  end

  defp http_patch(path, body) do
    case client() do
      nil -> {:error, :not_configured}
      client -> Tesla.patch(client, path, body) |> handle_response()
    end
  end

  defp http_delete(path) do
    case client() do
      nil -> {:error, :not_configured}
      client -> Tesla.delete(client, path) |> handle_response()
    end
  end

  defp handle_response({:ok, %Tesla.Env{status: status, body: body}}) when status in 200..299 do
    {:ok, body}
  end

  defp handle_response({:ok, %Tesla.Env{status: status, body: body}}) do
    Logger.error("[LemonSqueezy] API error: #{status} - #{inspect(body)}")
    {:error, {:api_error, status, body}}
  end

  defp handle_response({:error, reason}) do
    Logger.error("[LemonSqueezy] HTTP error: #{inspect(reason)}")
    {:error, {:http_error, reason}}
  end

  defp get_config(key) do
    config = Application.get_env(:query_service_ex, :lemon_squeezy, [])
    Keyword.get(config, key)
  end

  defp get_redirect_url do
    endpoint_config = Application.get_env(:query_service_ex, QueryServiceExWeb.Endpoint, [])
    url_config = Keyword.get(endpoint_config, :url, [])
    host = Keyword.get(url_config, :host, "localhost")
    "https://#{host}/billing/success"
  end
end
