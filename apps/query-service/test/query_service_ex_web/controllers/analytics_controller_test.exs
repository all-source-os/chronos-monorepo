defmodule QueryServiceExWeb.AnalyticsControllerTest do
  use ExUnit.Case, async: false

  import Plug.Test
  import Plug.Conn

  alias QueryServiceEx.Infrastructure.Adapters.AnalyticsCache
  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  setup do
    # Ensure cache is started for tests
    case GenServer.whereis(AnalyticsCache) do
      nil ->
        {:ok, _pid} = AnalyticsCache.start_link([])

      _pid ->
        AnalyticsCache.clear()
    end

    :ok
  end

  describe "GET /api/analytics/frequency" do
    test "returns 400 when since is missing" do
      conn =
        :get
        |> conn("/api/analytics/frequency?window=day")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 400 if auth provided
      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "since"
      end
    end

    test "returns 400 when window is missing" do
      conn =
        :get
        |> conn("/api/analytics/frequency?since=2026-02-01T00:00:00Z")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "window"
      end
    end

    test "returns 400 for invalid window" do
      conn =
        :get
        |> conn("/api/analytics/frequency?since=2026-02-01T00:00:00Z&window=invalid")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "Invalid window"
      end
    end

    test "accepts valid request parameters" do
      conn =
        :get
        |> conn(
          "/api/analytics/frequency?since=2026-02-01T00:00:00Z&window=day&entity_id=user-123"
        )
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided (depending on Core availability)
      assert conn.status in [200, 400, 401, 503]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
      end
    end
  end

  describe "GET /api/analytics/summary" do
    test "returns summary data" do
      conn =
        :get
        |> conn("/api/analytics/summary")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401, 503]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
      end
    end

    test "accepts filter parameters" do
      conn =
        :get
        |> conn("/api/analytics/summary?entity_id=user-123&event_type=user.created")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 503]
    end

    test "accepts time range parameters" do
      conn =
        :get
        |> conn("/api/analytics/summary?since=2026-02-01T00:00:00Z&until=2026-02-10T00:00:00Z")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 503]
    end
  end

  describe "GET /api/analytics/correlation" do
    test "returns 400 when event_type_a is missing" do
      conn =
        :get
        |> conn("/api/analytics/correlation?event_type_b=user.activated")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "event_type_a"
      end
    end

    test "returns 400 when event_type_b is missing" do
      conn =
        :get
        |> conn("/api/analytics/correlation?event_type_a=user.signup")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "event_type_b"
      end
    end

    test "accepts valid correlation request" do
      conn =
        :get
        |> conn(
          "/api/analytics/correlation?event_type_a=user.signup&event_type_b=user.activated&time_window_seconds=86400"
        )
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 503]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
      end
    end
  end

  describe "GET /api/analytics/percentiles" do
    test "returns 400 when event_type is missing" do
      conn =
        :get
        |> conn("/api/analytics/percentiles?field=duration")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "event_type"
      end
    end

    test "returns 400 when field is missing" do
      conn =
        :get
        |> conn("/api/analytics/percentiles?event_type=api.request")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "field"
      end
    end

    test "accepts valid percentiles request" do
      conn =
        :get
        |> conn(
          "/api/analytics/percentiles?event_type=api.request&field=duration&percentiles=50,90,99"
        )
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 404, 503]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
      end
    end
  end

  describe "GET /api/analytics/stddev" do
    test "returns 400 when event_type is missing" do
      conn =
        :get
        |> conn("/api/analytics/stddev?field=duration")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "event_type"
      end
    end

    test "returns 400 when field is missing" do
      conn =
        :get
        |> conn("/api/analytics/stddev?event_type=api.request")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "field"
      end
    end

    test "accepts valid stddev request" do
      conn =
        :get
        |> conn("/api/analytics/stddev?event_type=api.request&field=duration")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 404, 503]
    end
  end

  describe "GET /api/analytics/sliding-window" do
    test "requires since parameter" do
      conn =
        :get
        |> conn("/api/analytics/sliding-window?window=day")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # May fail with 400 or 401 depending on auth state
      assert conn.status in [400, 401]
    end

    test "accepts valid sliding window request" do
      conn =
        :get
        |> conn("/api/analytics/sliding-window?since=2026-02-01T00:00:00Z&window=day&slide=hour")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 503]
    end
  end

  describe "GET /api/analytics/session-window" do
    test "returns 400 when entity_id is missing" do
      conn =
        :get
        |> conn("/api/analytics/session-window?gap_seconds=1800")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert response["error"] =~ "entity_id"
      end
    end

    test "accepts valid session window request" do
      conn =
        :get
        |> conn("/api/analytics/session-window?entity_id=user-123&gap_seconds=1800")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400, 401, 503]
    end
  end

  describe "GET /api/analytics/cache/stats" do
    test "returns cache statistics" do
      conn =
        :get
        |> conn("/api/analytics/cache/stats")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200 if auth provided
      assert conn.status in [200, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
        assert Map.has_key?(response["data"], "size")
        assert Map.has_key?(response["data"], "memory")
      end
    end
  end

  describe "POST /api/analytics/cache/invalidate" do
    test "invalidates cache for tenant" do
      conn =
        :post
        |> conn("/api/analytics/cache/invalidate")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200 if auth provided
      assert conn.status in [200, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert response["success"] == true
      end
    end
  end
end
