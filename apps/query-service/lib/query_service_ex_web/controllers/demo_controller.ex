defmodule QueryServiceExWeb.DemoController do
  @moduledoc """
  Demo endpoints proxied to Core.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  def seed(conn, _params) do
    case RustCoreClient.seed_demo() do
      {:ok, body} ->
        json(conn, body)

      {:error, reason} ->
        conn
        |> put_status(:bad_gateway)
        |> json(%{error: "Failed to seed demo data: #{inspect(reason)}"})
    end
  end

  def feedback(conn, params) do
    event = %{
      event_type: "demo_feedback",
      entity_id: "demo-feedback",
      payload: %{
        positive: Map.get(params, "positive", true),
        page: Map.get(params, "page", "demo_zone"),
        timestamp: Map.get(params, "timestamp", DateTime.utc_now() |> DateTime.to_iso8601())
      }
    }

    case RustCoreClient.create_event(event) do
      {:ok, _body} ->
        json(conn, %{ok: true})

      {:error, _reason} ->
        # Still return success to the user — feedback is best-effort
        json(conn, %{ok: true})
    end
  end
end
