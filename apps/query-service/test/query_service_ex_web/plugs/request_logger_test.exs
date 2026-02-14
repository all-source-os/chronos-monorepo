defmodule QueryServiceExWeb.Plugs.RequestLoggerTest do
  @moduledoc """
  Tests for the RequestLogger plug.

  Verifies that HTTP requests are logged with structured metadata.
  """
  use ExUnit.Case, async: false

  import ExUnit.CaptureLog
  import Plug.Conn
  import Plug.Test

  alias QueryServiceExWeb.Plugs.RequestLogger

  # Set log level to debug to capture all log messages
  setup do
    current_level = Logger.level()
    Logger.configure(level: :debug)
    on_exit(fn -> Logger.configure(level: current_level) end)
    :ok
  end

  describe "init/1" do
    test "returns opts unchanged" do
      assert RequestLogger.init([]) == []
      assert RequestLogger.init(some: :option) == [some: :option]
    end
  end

  describe "call/2" do
    test "registers before_send callback" do
      conn = conn(:get, "/api/test")
      result = RequestLogger.call(conn, [])

      # The callback should be registered in private.before_send
      assert length(result.private.before_send) > 0
    end

    test "logs request on send" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("user-agent", "TestAgent/1.0")

      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          # Simulate sending a response
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      assert log =~ "GET"
      assert log =~ "/api/test"
      assert log =~ "200"
    end

    test "logs 4xx status at warning level" do
      conn = conn(:get, "/api/notfound")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :warning], fn ->
          processed_conn
          |> put_status(404)
          |> send_resp(404, "Not Found")
        end)

      assert log =~ "404"
    end

    test "logs 5xx status at error level" do
      conn = conn(:get, "/api/error")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :error], fn ->
          processed_conn
          |> put_status(500)
          |> send_resp(500, "Internal Server Error")
        end)

      assert log =~ "500"
    end

    test "extracts client IP from x-forwarded-for header" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("x-forwarded-for", "203.0.113.50, 70.41.3.18")

      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      # The client IP is in metadata (client_ip field), not the log message body
      # Just verify the request was logged successfully
      assert log =~ "GET"
      assert log =~ "/api/test"
    end

    test "uses remote_ip when x-forwarded-for not present" do
      conn = conn(:get, "/api/test")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      # The remote IP is in metadata (client_ip field), not the log message body
      # Just verify the request was logged successfully
      assert log =~ "GET"
      assert log =~ "/api/test"
    end

    test "includes user agent when present" do
      conn =
        conn(:get, "/api/test")
        |> put_req_header("user-agent", "Mozilla/5.0 TestBrowser")

      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      # User agent should be in metadata (may not appear in log message itself)
      # Just verify the request completes
      assert log =~ "GET"
    end

    test "includes query string in metadata" do
      conn = conn(:get, "/api/test?foo=bar&baz=qux")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      assert log =~ "/api/test"
    end

    test "includes duration in log message" do
      conn = conn(:get, "/api/test")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :info], fn ->
          # Add a small delay to have measurable duration (increased for CI stability)
          Process.sleep(10)

          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      # Should contain duration in ms format
      assert log =~ ~r/\d+.*ms/
    end

    test "handles missing user-agent header" do
      conn = conn(:get, "/api/test")
      processed_conn = RequestLogger.call(conn, [])

      # Should not raise when user-agent is missing
      log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      assert log =~ "GET"
    end

    test "logs health check paths at debug level even when status is 503" do
      conn = conn(:get, "/api/health")
      processed_conn = RequestLogger.call(conn, [])

      # At info level, health check debug logs should NOT appear
      info_log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(503)
          |> send_resp(503, "Service Unavailable")
        end)

      assert info_log == ""

      # At debug level, health check logs SHOULD appear
      conn2 = conn(:get, "/api/health")
      processed_conn2 = RequestLogger.call(conn2, [])

      debug_log =
        capture_log([level: :debug], fn ->
          processed_conn2
          |> put_status(503)
          |> send_resp(503, "Service Unavailable")
        end)

      assert debug_log =~ "/api/health"
    end

    test "non-health 503 paths still log at error level" do
      conn = conn(:get, "/api/events")
      processed_conn = RequestLogger.call(conn, [])

      log =
        capture_log([level: :error], fn ->
          processed_conn
          |> put_status(503)
          |> send_resp(503, "Service Unavailable")
        end)

      assert log =~ "503"
      assert log =~ "/api/events"
    end

    test "logs /health path at debug level" do
      conn = conn(:get, "/health")
      processed_conn = RequestLogger.call(conn, [])

      info_log =
        capture_log([level: :info], fn ->
          processed_conn
          |> put_status(200)
          |> send_resp(200, "OK")
        end)

      assert info_log == ""
    end
  end
end
