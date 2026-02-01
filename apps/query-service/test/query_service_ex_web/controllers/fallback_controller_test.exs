defmodule QueryServiceExWeb.FallbackControllerTest do
  @moduledoc """
  Tests for the FallbackController.

  Verifies that error tuples from controller actions are correctly
  translated into structured JSON error responses.
  """
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn
  import Phoenix.ConnTest, only: [json_response: 2]

  alias QueryServiceExWeb.FallbackController

  defp build_json_conn do
    conn(:get, "/test")
    |> put_private(:phoenix_format, "json")
    |> assign(:correlation_id, "test-correlation-id")
  end

  defp call_fallback(conn, error_tuple) do
    conn
    |> Plug.Conn.put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> FallbackController.call(error_tuple)
  end

  describe "call/2 with {:error, :not_found}" do
    test "returns 404 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :not_found})

      assert result.status == 404
    end

    test "returns structured error response" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :not_found})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "not_found"
      assert response["error"]["message"] == "Resource not found"
      assert response["error"]["correlation_id"] == "test-correlation-id"
      assert Map.has_key?(response["error"], "timestamp")
    end
  end

  describe "call/2 with {:error, :circuit_open}" do
    test "returns 503 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :circuit_open})

      assert result.status == 503
    end

    test "returns service unavailable message" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :circuit_open})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "service_unavailable"
      assert response["error"]["message"] =~ "temporarily unavailable"
    end
  end

  describe "call/2 with {:error, :timeout}" do
    test "returns 504 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :timeout})

      assert result.status == 504
    end

    test "returns timeout message" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :timeout})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "timeout"
      assert response["error"]["message"] == "Request timed out"
    end
  end

  describe "call/2 with {:error, :validation_failed, details}" do
    test "returns 422 status" do
      conn = build_json_conn()
      details = %{field: "name", error: "is required"}
      result = call_fallback(conn, {:error, :validation_failed, details})

      assert result.status == 422
    end

    test "includes validation details in response" do
      conn = build_json_conn()
      details = %{field: "name", error: "is required"}
      result = call_fallback(conn, {:error, :validation_failed, details})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "validation_failed"
      assert response["error"]["message"] == "Validation failed"
      assert response["error"]["details"]["field"] == "name"
      assert response["error"]["details"]["error"] == "is required"
    end
  end

  describe "call/2 with {:error, :unauthorized}" do
    test "returns 401 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :unauthorized})

      assert result.status == 401
    end

    test "returns authentication required message" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :unauthorized})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "unauthorized"
      assert response["error"]["message"] == "Authentication required"
    end
  end

  describe "call/2 with {:error, :forbidden}" do
    test "returns 403 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :forbidden})

      assert result.status == 403
    end

    test "returns access denied message" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :forbidden})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "forbidden"
      assert response["error"]["message"] == "Access denied"
    end
  end

  describe "call/2 with {:error, :bad_request, message}" do
    test "returns 400 status" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :bad_request, "Invalid parameter: foo"})

      assert result.status == 400
    end

    test "includes custom message in response" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :bad_request, "Invalid parameter: foo"})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "bad_request"
      assert response["error"]["message"] == "Invalid parameter: foo"
    end
  end

  describe "call/2 with {:error, %Tesla.Error{}}" do
    test "returns 502 status" do
      conn = build_json_conn()
      error = %Tesla.Error{reason: :econnrefused}
      result = call_fallback(conn, {:error, error})

      assert result.status == 502
    end

    test "returns backend error message" do
      conn = build_json_conn()
      error = %Tesla.Error{reason: :timeout}
      result = call_fallback(conn, {:error, error})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "backend_error"
      assert response["error"]["message"] == "Backend service unavailable"
    end
  end

  describe "call/2 with {:error, message} (binary)" do
    test "returns 502 for HTTP 5xx errors" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, "HTTP 503: Service Unavailable"})

      assert result.status == 502

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "backend_error"
      assert response["error"]["message"] == "Backend service error"
    end

    test "returns original status for HTTP 4xx errors" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, "HTTP 404: Not Found"})

      assert result.status == 404

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "backend_error"
    end

    test "returns 500 for non-HTTP error messages" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, "Something went wrong"})

      assert result.status == 500

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "internal_error"
      assert response["error"]["message"] == "Something went wrong"
    end
  end

  describe "call/2 with {:error, reason} (atom)" do
    test "handles :econnrefused as 503" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :econnrefused})

      assert result.status == 503

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "connection_refused"
      assert response["error"]["message"] == "Backend service unavailable"
    end

    test "handles :timeout as 504" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :timeout})

      assert result.status == 504

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "timeout"
    end

    test "handles :closed as 503" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :closed})

      assert result.status == 503

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "connection_closed"
    end

    test "handles :nxdomain as 503" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :nxdomain})

      assert result.status == 503

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "dns_error"
      assert response["error"]["message"] == "Cannot resolve backend service"
    end

    test "handles unknown atoms as 500" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :some_unknown_error})

      assert result.status == 500

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "some_unknown_error"
      assert response["error"]["message"] =~ "some_unknown_error"
    end
  end

  describe "call/2 with {:error, reason} (other types)" do
    test "handles map errors as 500" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, %{some: "data"}})

      assert result.status == 500

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "internal_error"
      assert response["error"]["message"] == "An unexpected error occurred"
    end

    test "handles tuple errors as 500" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, {:some, :tuple}})

      assert result.status == 500

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "internal_error"
    end
  end

  describe "correlation_id handling" do
    test "uses assigned correlation_id when available" do
      conn =
        conn(:get, "/test")
        |> put_private(:phoenix_format, "json")
        |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
        |> assign(:correlation_id, "custom-correlation-123")

      result = FallbackController.call(conn, {:error, :not_found})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["correlation_id"] == "custom-correlation-123"
    end

    test "uses 'unknown' when correlation_id not assigned" do
      conn =
        conn(:get, "/test")
        |> put_private(:phoenix_format, "json")
        |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)

      result = FallbackController.call(conn, {:error, :not_found})

      response = Jason.decode!(result.resp_body)
      assert response["error"]["correlation_id"] == "unknown"
    end
  end

  describe "timestamp format" do
    test "includes ISO8601 formatted timestamp" do
      conn = build_json_conn()
      result = call_fallback(conn, {:error, :not_found})

      response = Jason.decode!(result.resp_body)
      timestamp = response["error"]["timestamp"]

      assert {:ok, _datetime, _offset} = DateTime.from_iso8601(timestamp)
    end
  end
end
