defmodule QueryServiceExWeb.Plugs.JwtAuthTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Phoenix.ConnTest

  alias QueryServiceExWeb.Plugs.JwtAuth

  @secret "test-jwt-secret-for-unit-tests-1234"

  setup do
    # Ensure JWT_SECRET is set for tests
    original = System.get_env("JWT_SECRET")
    System.put_env("JWT_SECRET", @secret)

    # Ensure auth is not disabled
    original_auth = System.get_env("AUTH_DISABLED")
    System.delete_env("AUTH_DISABLED")

    on_exit(fn ->
      if original, do: System.put_env("JWT_SECRET", original), else: System.delete_env("JWT_SECRET")

      if original_auth,
        do: System.put_env("AUTH_DISABLED", original_auth),
        else: System.delete_env("AUTH_DISABLED")
    end)

    :ok
  end

  defp sign_jwt(claims, secret \\ @secret) do
    jwk = JOSE.JWK.from_oct(secret)
    jws = %{"alg" => "HS256"}
    {_, token} = JOSE.JWT.sign(jwk, jws, claims) |> JOSE.JWS.compact()
    token
  end

  describe "JwtAuth plug" do
    test "returns 401 when Authorization header is missing" do
      conn =
        build_conn()
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["code"] == "unauthorized"
      assert body["error"]["message"] =~ "missing Authorization"
    end

    test "returns 401 when Authorization header has wrong format" do
      conn =
        build_conn()
        |> put_req_header("authorization", "Basic abc123")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "invalid Authorization header format"
    end

    test "returns 401 when Bearer token is empty" do
      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer ")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "empty Bearer token"
    end

    test "returns 401 when JWT_SECRET is not configured" do
      System.delete_env("JWT_SECRET")

      token = sign_jwt(%{"sub" => "user-1", "tenant_id" => "t-1", "exp" => System.system_time(:second) + 3600})

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "not configured"
    end

    test "returns 401 when JWT signature is invalid (wrong secret)" do
      token = sign_jwt(
        %{"sub" => "user-1", "tenant_id" => "t-1", "exp" => System.system_time(:second) + 3600},
        "wrong-secret"
      )

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "invalid JWT signature"
    end

    test "returns 401 when token has expired" do
      token = sign_jwt(%{
        "sub" => "user-1",
        "tenant_id" => "t-1",
        "exp" => System.system_time(:second) - 60
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "expired"
    end

    test "returns 401 when sub claim is missing" do
      token = sign_jwt(%{
        "tenant_id" => "t-1",
        "exp" => System.system_time(:second) + 3600
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "missing sub"
    end

    test "returns 401 when tenant_id claim is missing" do
      token = sign_jwt(%{
        "sub" => "user-1",
        "exp" => System.system_time(:second) + 3600
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "missing tenant_id"
    end

    test "returns 401 when exp claim is missing" do
      token = sign_jwt(%{
        "sub" => "user-1",
        "tenant_id" => "t-1"
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401

      body = json_response(conn, 401)
      assert body["error"]["message"] =~ "missing exp"
    end

    test "sets assigns on valid JWT" do
      token = sign_jwt(%{
        "sub" => "user-42",
        "tenant_id" => "tenant-abc",
        "role" => "admin",
        "is_api_key" => false,
        "exp" => System.system_time(:second) + 3600
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      refute conn.halted
      assert conn.assigns.tenant_id == "tenant-abc"
      assert conn.assigns.user_id == "user-42"
      assert conn.assigns.role == "admin"
      assert conn.assigns.auth_method == :jwt
      assert conn.private[:allsource_tenant_id] == "tenant-abc"
    end

    test "sets assigns with nil role when role claim is absent" do
      token = sign_jwt(%{
        "sub" => "user-1",
        "tenant_id" => "tenant-1",
        "exp" => System.system_time(:second) + 3600
      })

      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer #{token}")
        |> JwtAuth.call(JwtAuth.init([]))

      refute conn.halted
      assert conn.assigns.tenant_id == "tenant-1"
      assert conn.assigns.user_id == "user-1"
      assert conn.assigns.role == nil
      assert conn.assigns.auth_method == :jwt
    end

    test "bypasses auth in dev mode" do
      System.put_env("AUTH_DISABLED", "true")

      conn =
        build_conn()
        |> JwtAuth.call(JwtAuth.init([]))

      refute conn.halted
    end

    test "returns 401 for malformed JWT token" do
      conn =
        build_conn()
        |> put_req_header("authorization", "Bearer not.a.jwt")
        |> JwtAuth.call(JwtAuth.init([]))

      assert conn.halted
      assert conn.status == 401
    end
  end
end
