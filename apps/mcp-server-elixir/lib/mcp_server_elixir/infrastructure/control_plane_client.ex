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
end
