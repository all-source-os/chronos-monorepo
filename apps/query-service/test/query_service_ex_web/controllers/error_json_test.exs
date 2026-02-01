defmodule QueryServiceExWeb.ErrorJSONTest do
  @moduledoc """
  Tests for the ErrorJSON view module.

  Verifies that error responses are correctly formatted for various HTTP status codes.
  """
  use ExUnit.Case, async: true

  alias QueryServiceExWeb.ErrorJSON

  describe "render/2 for 400.json" do
    test "returns bad_request error structure" do
      assigns = %{correlation_id: "test-400-id"}
      result = ErrorJSON.render("400.json", assigns)

      assert result.error.code == "bad_request"
      assert result.error.message == "Bad request"
      assert result.error.correlation_id == "test-400-id"
      assert Map.has_key?(result.error, :timestamp)
    end
  end

  describe "render/2 for 401.json" do
    test "returns unauthorized error structure" do
      assigns = %{correlation_id: "test-401-id"}
      result = ErrorJSON.render("401.json", assigns)

      assert result.error.code == "unauthorized"
      assert result.error.message == "Authentication required"
      assert result.error.correlation_id == "test-401-id"
    end
  end

  describe "render/2 for 403.json" do
    test "returns forbidden error structure" do
      assigns = %{correlation_id: "test-403-id"}
      result = ErrorJSON.render("403.json", assigns)

      assert result.error.code == "forbidden"
      assert result.error.message == "Access denied"
    end
  end

  describe "render/2 for 404.json" do
    test "returns not_found error structure" do
      assigns = %{correlation_id: "test-404-id"}
      result = ErrorJSON.render("404.json", assigns)

      assert result.error.code == "not_found"
      assert result.error.message == "Resource not found"
    end
  end

  describe "render/2 for 422.json" do
    test "returns unprocessable_entity error structure" do
      assigns = %{correlation_id: "test-422-id"}
      result = ErrorJSON.render("422.json", assigns)

      assert result.error.code == "unprocessable_entity"
      assert result.error.message == "Invalid request data"
    end
  end

  describe "render/2 for 500.json" do
    test "returns internal_server_error structure" do
      assigns = %{correlation_id: "test-500-id"}
      result = ErrorJSON.render("500.json", assigns)

      assert result.error.code == "internal_server_error"
      assert result.error.message == "An unexpected error occurred"
    end
  end

  describe "render/2 for 502.json" do
    test "returns bad_gateway error structure" do
      assigns = %{correlation_id: "test-502-id"}
      result = ErrorJSON.render("502.json", assigns)

      assert result.error.code == "bad_gateway"
      assert result.error.message == "Backend service unavailable"
    end
  end

  describe "render/2 for 503.json" do
    test "returns service_unavailable error structure" do
      assigns = %{correlation_id: "test-503-id"}
      result = ErrorJSON.render("503.json", assigns)

      assert result.error.code == "service_unavailable"
      assert result.error.message == "Service temporarily unavailable"
    end
  end

  describe "render/2 for 504.json" do
    test "returns gateway_timeout error structure" do
      assigns = %{correlation_id: "test-504-id"}
      result = ErrorJSON.render("504.json", assigns)

      assert result.error.code == "gateway_timeout"
      assert result.error.message == "Request timed out"
    end
  end

  describe "render/2 for unknown templates" do
    test "handles unknown status code template" do
      assigns = %{correlation_id: "test-unknown-id"}
      result = ErrorJSON.render("418.json", assigns)

      assert result.error.code == "418"
      # Phoenix will provide a message based on status
      assert is_binary(result.error.message)
    end
  end

  describe "correlation_id extraction" do
    test "extracts from conn.assigns.correlation_id" do
      assigns = %{
        conn: %{assigns: %{correlation_id: "from-conn-assigns"}}
      }

      result = ErrorJSON.render("404.json", assigns)

      assert result.error.correlation_id == "from-conn-assigns"
    end

    test "extracts from top-level correlation_id" do
      assigns = %{correlation_id: "from-top-level"}
      result = ErrorJSON.render("404.json", assigns)

      assert result.error.correlation_id == "from-top-level"
    end

    test "uses 'unknown' when correlation_id not found" do
      assigns = %{}
      result = ErrorJSON.render("404.json", assigns)

      assert result.error.correlation_id == "unknown"
    end

    test "prefers conn.assigns over top-level" do
      assigns = %{
        conn: %{assigns: %{correlation_id: "from-conn"}},
        correlation_id: "from-top-level"
      }

      result = ErrorJSON.render("404.json", assigns)

      assert result.error.correlation_id == "from-conn"
    end
  end

  describe "timestamp format" do
    test "includes ISO8601 formatted timestamp" do
      assigns = %{correlation_id: "test-timestamp"}
      result = ErrorJSON.render("404.json", assigns)

      timestamp = result.error.timestamp
      assert {:ok, _datetime, _offset} = DateTime.from_iso8601(timestamp)
    end
  end
end
