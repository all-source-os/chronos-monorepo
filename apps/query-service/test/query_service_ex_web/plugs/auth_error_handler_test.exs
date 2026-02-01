defmodule QueryServiceExWeb.Plugs.AuthErrorHandlerTest do
  @moduledoc """
  Tests for the AuthErrorHandler plug.

  Verifies that Guardian authentication errors are correctly translated
  into structured JSON error responses.
  """
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias QueryServiceExWeb.Plugs.AuthErrorHandler

  defp build_conn do
    conn(:get, "/api/protected")
    |> put_private(:phoenix_format, "json")
    |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> assign(:correlation_id, "test-correlation-123")
  end

  describe "auth_error/3" do
    test "handles :unauthenticated error" do
      conn = build_conn()
      result = AuthErrorHandler.auth_error(conn, {:unauthenticated, :no_token}, [])

      assert result.status == 401

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "unauthenticated"
      assert response["error"]["message"] == "Authentication required"
      assert response["error"]["correlation_id"] == "test-correlation-123"
      assert Map.has_key?(response["error"], "timestamp")
    end

    test "handles :invalid_token error" do
      conn = build_conn()
      result = AuthErrorHandler.auth_error(conn, {:invalid_token, :token_expired}, [])

      assert result.status == 401

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "invalid_token"
      assert response["error"]["message"] == "Invalid or expired token"
    end

    test "handles :no_resource_found error" do
      conn = build_conn()
      result = AuthErrorHandler.auth_error(conn, {:no_resource_found, :user_deleted}, [])

      assert result.status == 401

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "user_not_found"
      assert response["error"]["message"] == "User not found"
    end

    test "handles unknown error types" do
      conn = build_conn()
      result = AuthErrorHandler.auth_error(conn, {:some_unknown_error, :reason}, [])

      assert result.status == 401

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "unauthorized"
      assert response["error"]["message"] == "Authentication failed"
    end

    test "uses 'unknown' correlation_id when not assigned" do
      conn =
        conn(:get, "/api/protected")
        |> put_private(:phoenix_format, "json")
        |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)

      result = AuthErrorHandler.auth_error(conn, {:unauthenticated, :no_token}, [])

      response = Jason.decode!(result.resp_body)
      assert response["error"]["correlation_id"] == "unknown"
    end

    test "returns ISO8601 formatted timestamp" do
      conn = build_conn()
      result = AuthErrorHandler.auth_error(conn, {:unauthenticated, :reason}, [])

      response = Jason.decode!(result.resp_body)
      timestamp = response["error"]["timestamp"]

      assert {:ok, _datetime, _offset} = DateTime.from_iso8601(timestamp)
    end

    test "ignores opts parameter" do
      conn = build_conn()

      # Should work the same with any opts
      result1 = AuthErrorHandler.auth_error(conn, {:unauthenticated, :reason}, [])
      result2 = AuthErrorHandler.auth_error(conn, {:unauthenticated, :reason}, some: :option)

      response1 = Jason.decode!(result1.resp_body)
      response2 = Jason.decode!(result2.resp_body)

      assert response1["error"]["code"] == response2["error"]["code"]
      assert response1["error"]["message"] == response2["error"]["message"]
    end
  end
end
