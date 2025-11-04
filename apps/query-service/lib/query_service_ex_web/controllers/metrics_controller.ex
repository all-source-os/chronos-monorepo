defmodule QueryServiceExWeb.MetricsController do
  @moduledoc """
  Controller for system metrics and monitoring.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  Get system metrics.

  Returns metrics from the backend and Elixir runtime.
  """
  def show(conn, _params) do
    # Get backend metrics
    backend_metrics =
      case RustCoreClient.get_metrics() do
        {:ok, metrics} -> metrics
        {:error, _} -> %{error: "Backend unavailable"}
      end

    # Get Elixir runtime metrics
    elixir_metrics = %{
      processes: Process.list() |> length(),
      memory: :erlang.memory() |> format_memory(),
      uptime_seconds: :erlang.statistics(:wall_clock) |> elem(0) |> div(1000),
      schedulers: :erlang.system_info(:schedulers_online)
    }

    response = %{
      service: "query_service_ex",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601(),
      elixir: elixir_metrics,
      backend: backend_metrics
    }

    json(conn, response)
  end

  defp format_memory(memory_list) do
    %{
      total_mb: memory_list[:total] |> bytes_to_mb(),
      processes_mb: memory_list[:processes] |> bytes_to_mb(),
      atom_mb: memory_list[:atom] |> bytes_to_mb(),
      binary_mb: memory_list[:binary] |> bytes_to_mb(),
      ets_mb: memory_list[:ets] |> bytes_to_mb()
    }
  end

  defp bytes_to_mb(bytes), do: Float.round(bytes / 1024 / 1024, 2)
end
