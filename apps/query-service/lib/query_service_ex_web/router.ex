defmodule QueryServiceExWeb.Router do
  @moduledoc """
  Phoenix router for Query Service API endpoints.
  """

  use Phoenix.Router

  import Plug.Conn
  import Phoenix.Controller

  pipeline :api do
    plug(:accepts, ["json"])
  end

  scope "/api", QueryServiceExWeb do
    pipe_through(:api)

    # Health check
    get("/health", HealthController, :show)

    # Events endpoints
    get("/events", EventController, :index)
    get("/events/:id", EventController, :show)
    post("/events", EventController, :create)
    post("/events/batch", EventController, :create_batch)
    get("/events/entity/:entity_id", EventController, :by_entity)
    get("/events/type/:event_type", EventController, :by_type)

    # Query execution
    post("/query", QueryController, :execute)

    # Projections endpoints
    get("/projections", ProjectionController, :index)
    get("/projections/:name", ProjectionController, :show)
    post("/projections", ProjectionController, :create)
    delete("/projections/:name", ProjectionController, :delete)
    get("/projections/:name/state", ProjectionController, :get_state)
    post("/projections/:name/reset", ProjectionController, :reset)

    # Schemas endpoints
    get("/schemas", SchemaController, :index)
    get("/schemas/:event_type", SchemaController, :show)
    post("/schemas", SchemaController, :register)

    # Metrics
    get("/metrics", MetricsController, :show)
  end
end
