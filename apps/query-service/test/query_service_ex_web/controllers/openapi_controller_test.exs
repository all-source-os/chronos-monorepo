defmodule QueryServiceExWeb.OpenApiControllerTest do
  use QueryServiceExWeb.ConnCase, async: true

  describe "GET /api/openapi" do
    test "returns OpenAPI specification in JSON format", %{conn: conn} do
      conn = get(conn, "/api/openapi")

      assert json_response(conn, 200)
      assert get_resp_header(conn, "content-type") |> hd() =~ "application/json"

      spec = json_response(conn, 200)

      # Verify OpenAPI structure
      assert spec["openapi"] =~ "3."
      assert spec["info"]["title"] == "Query Service API"
      assert is_binary(spec["info"]["version"])
      assert is_map(spec["paths"])
      assert is_map(spec["components"])
    end

    test "includes security schemes", %{conn: conn} do
      conn = get(conn, "/api/openapi")
      spec = json_response(conn, 200)

      security_schemes = spec["components"]["securitySchemes"]
      assert is_map(security_schemes)
      assert Map.has_key?(security_schemes, "bearer_auth")
      assert security_schemes["bearer_auth"]["type"] == "http"
      assert security_schemes["bearer_auth"]["scheme"] == "bearer"
    end

    test "includes all expected paths", %{conn: conn} do
      conn = get(conn, "/api/openapi")
      spec = json_response(conn, 200)

      paths = Map.keys(spec["paths"])

      # Health endpoints
      assert "/api/health" in paths
      assert "/api/health/live" in paths
      assert "/api/health/ready" in paths

      # Metrics endpoints
      assert "/api/metrics" in paths
      assert "/api/metrics/prometheus" in paths

      # Documentation endpoints
      assert "/api/openapi" in paths
      assert "/api/docs" in paths

      # Events endpoints
      assert "/api/events" in paths

      # Query endpoint
      assert "/api/query" in paths

      # Projections endpoints
      assert "/api/projections" in paths
    end

    test "includes tags for grouping", %{conn: conn} do
      conn = get(conn, "/api/openapi")
      spec = json_response(conn, 200)

      tags = spec["tags"]
      assert is_list(tags)

      tag_names = Enum.map(tags, & &1["name"])
      assert "Health" in tag_names
      assert "Metrics" in tag_names
      assert "Events" in tag_names
      assert "Query" in tag_names
      assert "Projections" in tag_names
      assert "Authentication" in tag_names
    end
  end

  describe "GET /api/docs" do
    test "returns Swagger UI HTML page", %{conn: conn} do
      conn = get(conn, "/api/docs")

      assert response(conn, 200)
      assert get_resp_header(conn, "content-type") |> hd() =~ "text/html"

      body = response(conn, 200)
      assert body =~ "swagger-ui"
      assert body =~ "SwaggerUIBundle"
      assert body =~ "/api/openapi"
    end

    test "HTML includes required Swagger UI assets", %{conn: conn} do
      conn = get(conn, "/api/docs")
      body = response(conn, 200)

      # Check for CSS
      assert body =~ "swagger-ui.css"

      # Check for JS
      assert body =~ "swagger-ui-bundle.js"
      assert body =~ "swagger-ui-standalone-preset.js"
    end
  end
end
