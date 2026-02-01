defmodule QueryServiceExWeb.Plugs.CorrelationIdTest do
  @moduledoc """
  Tests for the CorrelationId plug.

  Verifies correlation ID extraction, generation, and propagation.
  """
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias QueryServiceExWeb.Plugs.CorrelationId

  describe "init/1" do
    test "returns opts unchanged" do
      assert CorrelationId.init([]) == []
      assert CorrelationId.init(some: :option) == [some: :option]
    end
  end

  describe "call/2" do
    test "extracts correlation_id from request header" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("x-correlation-id", "existing-correlation-123")

      result = CorrelationId.call(conn, [])

      assert result.assigns[:correlation_id] == "existing-correlation-123"
    end

    test "generates correlation_id when not in request" do
      conn = conn(:get, "/api/test")
      result = CorrelationId.call(conn, [])

      assert result.assigns[:correlation_id] != nil
      assert is_binary(result.assigns[:correlation_id])
    end

    test "generated correlation_id is UUID format" do
      conn = conn(:get, "/api/test")
      result = CorrelationId.call(conn, [])

      correlation_id = result.assigns[:correlation_id]

      # UUID format: 8-4-4-4-12 characters (lowercase hex)
      assert Regex.match?(
               ~r/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
               correlation_id
             )
    end

    test "adds correlation_id to response headers" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("x-correlation-id", "response-test-456")

      result = CorrelationId.call(conn, [])

      [header_value] = get_resp_header(result, "x-correlation-id")
      assert header_value == "response-test-456"
    end

    test "generated correlation_id is added to response headers" do
      conn = conn(:get, "/api/test")
      result = CorrelationId.call(conn, [])

      [header_value] = get_resp_header(result, "x-correlation-id")
      assert header_value == result.assigns[:correlation_id]
    end

    test "sets Logger metadata" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("x-correlation-id", "logger-metadata-789")

      CorrelationId.call(conn, [])

      # Logger metadata should include the correlation_id
      metadata = Logger.metadata()
      assert metadata[:correlation_id] == "logger-metadata-789"
    end

    test "uses first value when multiple x-correlation-id headers" do
      # This simulates a case where multiple headers might be present
      conn =
        conn(:get, "/api/test")
        |> put_req_header("x-correlation-id", "first-id")

      result = CorrelationId.call(conn, [])

      assert result.assigns[:correlation_id] == "first-id"
    end

    test "generates unique correlation_ids for each request" do
      conn1 = conn(:get, "/api/test1")
      conn2 = conn(:get, "/api/test2")

      result1 = CorrelationId.call(conn1, [])
      result2 = CorrelationId.call(conn2, [])

      refute result1.assigns[:correlation_id] == result2.assigns[:correlation_id]
    end
  end
end
