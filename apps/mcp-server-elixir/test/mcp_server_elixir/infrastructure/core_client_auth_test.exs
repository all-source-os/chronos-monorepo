defmodule McpServerElixir.Infrastructure.CoreClientAuthTest do
  @moduledoc """
  Regression tests for #228 — remote mode against a hosted, authenticated Core.

  Two defects were confirmed:

    1. the HTTP client sent no `Authorization` header at all → a hosted Core
       401s → the MCP layer reports JSON-RPC `-32603 Internal error`
    2. the WebSocket client sent the raw key instead of `Bearer <key>` → same 401

  The issue also reported that `:core_url` was frozen at build time by the
  module-level `plug`. That does NOT reproduce on Tesla 1.18: `plug` arguments are
  compiled into `__middleware__/0` and re-evaluated per request. The
  "core_url resolution" tests below pin that behaviour so a future Tesla upgrade
  or refactor that *does* freeze it fails here instead of in production.
  """
  use ExUnit.Case, async: false

  alias McpServerElixir.Infrastructure.CoreClient
  alias McpServerElixir.Infrastructure.CoreWebSocketClient

  setup do
    original_url = Application.get_env(:mcp_server_elixir, :core_url)
    original_key = Application.get_env(:mcp_server_elixir, :core_api_key)

    on_exit(fn ->
      restore(:core_url, original_url)
      restore(:core_api_key, original_key)
    end)

    :ok
  end

  defp restore(key, nil), do: Application.delete_env(:mcp_server_elixir, key)
  defp restore(key, value), do: Application.put_env(:mcp_server_elixir, key, value)

  # Pull the values back out of the middleware stack Tesla builds for a request.
  defp base_url do
    Enum.find_value(CoreClient.__middleware__(), fn
      {Tesla.Middleware.BaseUrl, :call, [url]} -> url
      _ -> nil
    end)
  end

  defp auth_header do
    Enum.find_value(CoreClient.__middleware__(), fn
      {Tesla.Middleware.Headers, :call, [headers]} ->
        Enum.find_value(headers, fn
          {"authorization", value} -> value
          _ -> nil
        end)

      _ ->
        nil
    end)
  end

  describe "core_url resolution (reported as build-time-frozen; pinned here)" do
    test "base URL is read at call time, not frozen at compile time" do
      Application.put_env(:mcp_server_elixir, :core_url, "https://api.all-source.xyz")
      assert base_url() == "https://api.all-source.xyz"

      # Same module, same beam — a second value must be picked up, which a
      # compile-time `plug` argument could not do.
      Application.put_env(:mcp_server_elixir, :core_url, "http://other-core:3900")
      assert base_url() == "http://other-core:3900"
    end

    test "falls back to the localhost default when unconfigured" do
      Application.delete_env(:mcp_server_elixir, :core_url)
      assert base_url() == "http://localhost:3900"
    end
  end

  describe "HTTP Authorization header (confirmed bug 1: no auth middleware)" do
    test "attaches the API key as a Bearer token" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "as_live_abc123")

      assert auth_header() == "Bearer as_live_abc123"
    end

    test "does not double-prefix a key that already carries the scheme" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "Bearer as_live_abc123")

      assert auth_header() == "Bearer as_live_abc123"
    end

    test "sends no Authorization header when no key is configured" do
      Application.delete_env(:mcp_server_elixir, :core_api_key)
      assert CoreClient.auth_headers() == []
      assert auth_header() == nil
    end

    test "treats an empty key as unset" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "")
      assert CoreClient.auth_headers() == []
      assert auth_header() == nil
    end

    test "the key is picked up at call time" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "key-one")
      assert auth_header() == "Bearer key-one"

      Application.put_env(:mcp_server_elixir, :core_api_key, "key-two")
      assert auth_header() == "Bearer key-two"
    end
  end

  describe "WebSocket Authorization header (confirmed bug 2: raw key, no scheme)" do
    test "sends the key with the Bearer scheme" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "as_live_abc123")

      assert CoreWebSocketClient.build_auth_headers() == [
               {"Authorization", "Bearer as_live_abc123"}
             ]
    end

    test "does not double-prefix an already-scheme'd key" do
      Application.put_env(:mcp_server_elixir, :core_api_key, "Bearer as_live_abc123")

      assert CoreWebSocketClient.build_auth_headers() == [
               {"Authorization", "Bearer as_live_abc123"}
             ]
    end

    test "sends no header when the key is unset or empty" do
      Application.delete_env(:mcp_server_elixir, :core_api_key)
      assert CoreWebSocketClient.build_auth_headers() == []

      Application.put_env(:mcp_server_elixir, :core_api_key, "")
      assert CoreWebSocketClient.build_auth_headers() == []
    end
  end
end
