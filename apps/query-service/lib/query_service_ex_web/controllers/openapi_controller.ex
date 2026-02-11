defmodule QueryServiceExWeb.OpenApiController do
  @moduledoc """
  Controller for serving OpenAPI specification and Swagger UI.

  Provides:
  - GET /api/openapi - Returns the OpenAPI 3.0 specification in JSON format
  - GET /api/docs - Serves Swagger UI for interactive API documentation
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias OpenApiSpex.OpenApi
  alias QueryServiceExWeb.ApiSpec

  tags(["Documentation"])

  operation(:spec,
    summary: "Get OpenAPI specification",
    description: "Returns the OpenAPI 3.0 specification in JSON format.",
    responses: [
      ok: {"OpenAPI specification", "application/json", nil}
    ]
  )

  @doc """
  Returns the OpenAPI specification as JSON.

  GET /api/openapi
  """
  def spec(conn, _params) do
    spec = ApiSpec.spec()

    conn
    |> put_resp_content_type("application/json")
    |> json(OpenApi.to_map(spec))
  end

  operation(:docs,
    summary: "Swagger UI",
    description: "Serves Swagger UI for interactive API documentation.",
    responses: [
      ok: {"Swagger UI HTML", "text/html", nil}
    ]
  )

  @doc """
  Serves Swagger UI for interactive API documentation.

  GET /api/docs
  """
  def docs(conn, _params) do
    html = """
    <!DOCTYPE html>
    <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Query Service API Documentation</title>
      <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
      <style>
        html { box-sizing: border-box; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin: 0; background: #fafafa; }
        .swagger-ui .topbar { display: none; }
        .swagger-ui .info { margin-top: 20px; }
        .swagger-ui .info .title { font-size: 36px; }
      </style>
    </head>
    <body>
      <div id="swagger-ui"></div>
      <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
      <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
      <script>
        window.onload = function() {
          window.ui = SwaggerUIBundle({
            url: "/api/openapi",
            dom_id: '#swagger-ui',
            deepLinking: true,
            presets: [
              SwaggerUIBundle.presets.apis,
              SwaggerUIStandalonePreset
            ],
            plugins: [
              SwaggerUIBundle.plugins.DownloadUrl
            ],
            layout: "StandaloneLayout",
            persistAuthorization: true,
            displayRequestDuration: true,
            filter: true,
            tryItOutEnabled: true
          });
        };
      </script>
    </body>
    </html>
    """

    conn
    |> put_resp_content_type("text/html")
    |> send_resp(200, html)
  end
end
