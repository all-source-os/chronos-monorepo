defmodule QueryServiceExWeb.ConfigController do
  @moduledoc """
  Serves configuration data that can be updated without recompile.
  """

  use Phoenix.Controller, formats: [:json]

  @benchmarks_path "priv/config/benchmarks.json"

  def benchmarks(conn, _params) do
    path = Application.app_dir(:query_service_ex, @benchmarks_path)

    case File.read(path) do
      {:ok, contents} ->
        case Jason.decode(contents) do
          {:ok, data} ->
            json(conn, data)

          {:error, _reason} ->
            conn
            |> put_status(:internal_server_error)
            |> json(%{error: "Invalid benchmarks config format"})
        end

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{error: "Benchmarks config not found"})
    end
  end
end
