defmodule QueryServiceExWeb.Plugs.ErrorHandlerTest do
  @moduledoc """
  Tests for the ErrorHandler plug.

  Verifies that exceptions are caught and returned as structured
  JSON error responses with proper status codes and telemetry.
  """
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias QueryServiceExWeb.Plugs.ErrorHandler

  describe "init/1" do
    test "returns opts unchanged" do
      assert ErrorHandler.init([]) == []
      assert ErrorHandler.init(some: :option) == [some: :option]
    end
  end

  describe "call/2 without exception" do
    test "passes connection through unchanged" do
      conn = conn(:get, "/test")
      result = ErrorHandler.call(conn, [])

      assert result == conn
      refute result.halted
    end
  end

  describe "handle_exception/3" do
    test "handles Plug.Parsers.ParseError with 400 status" do
      conn =
        conn(:post, "/test")
        |> assign(:correlation_id, "test-correlation-123")

      exception = %Plug.Parsers.ParseError{exception: %RuntimeError{message: "parse error"}}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 400
      assert result.halted

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "parse_error"
      assert response["error"]["message"] == "Invalid request body"
      assert response["error"]["correlation_id"] == "test-correlation-123"
      assert Map.has_key?(response["error"], "timestamp")
    end

    test "handles Plug.Parsers.UnsupportedMediaTypeError with 415 status" do
      conn =
        conn(:post, "/test")
        |> assign(:correlation_id, "test-correlation-456")

      exception = %Plug.Parsers.UnsupportedMediaTypeError{media_type: "text/plain"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 415

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "unsupported_media_type"
      assert response["error"]["message"] == "Content-Type not supported"
    end

    test "handles Phoenix.Router.NoRouteError with 404 status" do
      conn =
        conn(:get, "/unknown")
        |> assign(:correlation_id, "test-correlation-789")

      exception = %Phoenix.Router.NoRouteError{
        conn: conn,
        router: QueryServiceExWeb.Router,
        plug_status: 404
      }

      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 404

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "not_found"
      assert response["error"]["message"] == "Route not found"
    end

    test "handles ArgumentError with 400 status and custom message" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "arg-error-123")

      exception = %ArgumentError{message: "invalid argument: expected integer"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 400

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "argument_error"
      assert response["error"]["message"] == "invalid argument: expected integer"
    end

    test "handles FunctionClauseError with 400 status" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "func-clause-123")

      exception = %FunctionClauseError{
        module: SomeModule,
        function: :some_func,
        arity: 2,
        args: [1, 2]
      }

      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 400

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "bad_request"
      assert response["error"]["message"] == "Invalid request parameters"
    end

    test "handles Jason.DecodeError with 400 status" do
      conn =
        conn(:post, "/test")
        |> assign(:correlation_id, "json-decode-123")

      exception = %Jason.DecodeError{data: "invalid", position: 0}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 400

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "json_decode_error"
      assert response["error"]["message"] == "Invalid JSON in request body"
    end

    test "handles DBConnection.ConnectionError with 503 status" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "db-conn-123")

      exception = %DBConnection.ConnectionError{
        message: "connection refused",
        severity: :error,
        reason: :closed
      }

      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 503

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "database_unavailable"
      assert response["error"]["message"] == "Database connection unavailable"
    end

    test "handles unknown exceptions with 500 status" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "unknown-error-123")

      exception = %RuntimeError{message: "something went wrong"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.status == 500

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "internal_server_error"
      assert response["error"]["message"] == "An unexpected error occurred"
    end

    test "uses 'unknown' correlation_id when not assigned" do
      conn = conn(:get, "/test")

      exception = %RuntimeError{message: "error"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      response = Jason.decode!(result.resp_body)
      assert response["error"]["correlation_id"] == "unknown"
    end

    test "sets correct content-type header" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "content-type-test")

      exception = %RuntimeError{message: "error"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      [content_type | _] = get_resp_header(result, "content-type")
      assert content_type =~ "application/json"
    end

    test "halts the connection" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "halt-test")

      exception = %RuntimeError{message: "error"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      assert result.halted
    end

    test "returns ISO8601 timestamp" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "timestamp-test")

      exception = %RuntimeError{message: "error"}
      result = ErrorHandler.handle_exception(conn, exception, [])

      response = Jason.decode!(result.resp_body)
      timestamp = response["error"]["timestamp"]

      assert {:ok, _datetime, _offset} = DateTime.from_iso8601(timestamp)
    end
  end

  describe "telemetry emission" do
    setup do
      ref = make_ref()
      test_pid = self()

      :telemetry.attach(
        "test-error-handler-#{inspect(ref)}",
        [:query_service_ex, :http, :error],
        fn event, measurements, metadata, _config ->
          send(test_pid, {:telemetry_event, event, measurements, metadata})
        end,
        nil
      )

      on_exit(fn ->
        :telemetry.detach("test-error-handler-#{inspect(ref)}")
      end)

      :ok
    end

    test "emits telemetry on exception" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "telemetry-test-123")

      exception = %RuntimeError{message: "error"}
      ErrorHandler.handle_exception(conn, exception, [])

      assert_receive {:telemetry_event, [:query_service_ex, :http, :error], %{}, metadata}
      assert metadata.correlation_id == "telemetry-test-123"
      assert metadata.error_type == "Elixir.RuntimeError"
      assert metadata.status == 500
    end

    test "emits telemetry with correct status for client errors" do
      conn =
        conn(:get, "/test")
        |> assign(:correlation_id, "client-error-telemetry")

      exception = %ArgumentError{message: "bad arg"}
      ErrorHandler.handle_exception(conn, exception, [])

      assert_receive {:telemetry_event, [:query_service_ex, :http, :error], %{}, metadata}
      assert metadata.status == 400
    end
  end
end
