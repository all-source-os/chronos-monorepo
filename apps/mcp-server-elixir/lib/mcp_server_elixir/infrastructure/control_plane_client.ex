defmodule McpServerElixir.Infrastructure.ControlPlaneClient do
  @moduledoc """
  HTTP client for communicating with the Go Control Plane API.

  Provides methods for cluster management and status monitoring.
  """

  use Tesla

  @default_base_url "http://localhost:3901"
  @default_timeout 10_000

  plug(
    Tesla.Middleware.BaseUrl,
    Application.get_env(:mcp_server_elixir, :control_url, @default_base_url)
  )

  plug(Tesla.Middleware.JSON)
  plug(Tesla.Middleware.Timeout, timeout: @default_timeout)

  plug(Tesla.Middleware.Retry,
    delay: 100,
    max_retries: 2,
    max_delay: 1_000,
    should_retry: fn
      {:ok, %{status: status}} when status in [408, 429, 500, 502, 503, 504] -> true
      {:ok, _} -> false
      {:error, _} -> true
    end
  )

  def new do
    %{client: __MODULE__}
  end

  @doc "Get cluster status and health information"
  def get_cluster_status(_client) do
    case get("/api/v1/cluster/status") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Tenant Management Endpoints
  # ============================================================================

  @doc "Create a new tenant with quotas and settings"
  def tenant_create(_client, params) do
    case post("/api/v1/tenants", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Update tenant settings and quotas"
  def tenant_update(_client, tenant_id, params) do
    case put("/api/v1/tenants/#{tenant_id}", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get usage statistics for a tenant"
  def tenant_usage(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/usage", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get or update quota configuration for a tenant"
  def tenant_quotas(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/quotas", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Suspend a tenant (soft disable)"
  def tenant_suspend(_client, tenant_id, params) do
    case post("/api/v1/tenants/#{tenant_id}/suspend", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Export all data for a tenant"
  def tenant_export(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/export", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
