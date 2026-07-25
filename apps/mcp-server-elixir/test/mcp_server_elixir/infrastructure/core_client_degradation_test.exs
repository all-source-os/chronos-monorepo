defmodule McpServerElixir.Infrastructure.CoreClientDegradationTest do
  @moduledoc """
  Regression tests for #229 — every aggregate/state tool failed with a bare
  `-32603` against the hosted gateway while raw event reads worked.

  Root cause: the connector calls Core's native API, but `GET /api/v1/stats` is a
  **global, whole-store** endpoint (`store.stats()`) that the gateway deliberately
  does not route — a tenant must not see cross-tenant totals. Same for
  `/api/v1/entities/{id}/state`. Those paths 404 at the gateway, so `get_stats`,
  `quick_stats` (unfiltered), `storage_stats`, `reconstruct_state` and
  `explain_entity` all failed, while `query_events` (which IS proxied) worked.

  The fix degrades to the tenant's own event stream when a route is absent, and
  deliberately does NOT degrade on 401/403 — masking a credential problem behind
  an approximate answer is what made this hard to diagnose in the first place.
  """
  use ExUnit.Case, async: false

  alias McpServerElixir.Infrastructure.CoreClient

  @events [
    %{
      "id" => "1",
      "event_type" => "created",
      "entity_id" => "e1",
      "timestamp" => "2026-07-01T00:00:00Z",
      "payload" => %{"colour" => "red", "size" => 1}
    },
    %{
      "id" => "2",
      "event_type" => "updated",
      "entity_id" => "e1",
      "timestamp" => "2026-07-02T00:00:00Z",
      "payload" => %{"colour" => "blue"}
    },
    %{
      "id" => "3",
      "event_type" => "created",
      "entity_id" => "e2",
      "timestamp" => "2026-07-03T00:00:00Z",
      "payload" => %{"colour" => "green"}
    }
  ]

  setup do
    original_url = Application.get_env(:mcp_server_elixir, :core_url)
    on_exit(fn -> Application.put_env(:mcp_server_elixir, :core_url, original_url) end)
    :ok
  end

  # A stand-in gateway. `non_query_status` is what it returns for anything other
  # than /api/v1/events/query — 404 mirrors the real control plane, which has no
  # route registered for /stats or /entities/*.
  defp start_gateway(non_query_status) do
    {:ok, listen} =
      :gen_tcp.listen(0, [
        :binary,
        packet: :raw,
        active: false,
        reuseaddr: true,
        ip: {127, 0, 0, 1}
      ])

    {:ok, port} = :inet.port(listen)
    parent = self()

    spawn_link(fn -> accept_loop(listen, non_query_status, parent) end)
    Application.put_env(:mcp_server_elixir, :core_url, "http://127.0.0.1:#{port}")
    port
  end

  defp accept_loop(listen, status, parent) do
    case :gen_tcp.accept(listen) do
      {:ok, sock} ->
        spawn(fn -> serve(sock, status, parent) end)
        accept_loop(listen, status, parent)

      _ ->
        :ok
    end
  end

  defp serve(sock, status, parent) do
    with {:ok, data} <- :gen_tcp.recv(sock, 0, 5_000) do
      [request_line | _] = String.split(data, "\r\n")
      [_method, path | _] = String.split(request_line, " ")
      send(parent, {:requested, path})

      {code, body} =
        if String.starts_with?(path, "/api/v1/events/query") do
          {200, Jason.encode!(%{"events" => events_for(path), "count" => length(events_for(path))})}
        else
          {status, Jason.encode!(%{"error" => "no route"})}
        end

      :gen_tcp.send(sock, [
        "HTTP/1.1 #{code} X\r\n",
        "content-type: application/json\r\n",
        "content-length: #{byte_size(body)}\r\n\r\n",
        body
      ])
    end

    :gen_tcp.close(sock)
  end

  # Mirror the gateway's tenant-scoped filtering: honour entity_id when present.
  defp events_for(path) do
    case Regex.run(~r/entity_id=([^&]+)/, path) do
      [_, entity_id] -> Enum.filter(@events, &(&1["entity_id"] == entity_id))
      nil -> @events
    end
  end

  describe "get_stats/0 when the gateway does not route /api/v1/stats" do
    test "derives Core's stats shape from the tenant's events" do
      start_gateway(404)

      assert {:ok, stats} = CoreClient.get_stats()

      assert stats["total_events"] == 3
      assert stats["total_entities"] == 2
      assert stats["total_event_types"] == 2
      assert stats["event_types"] == %{"created" => 2, "updated" => 1}
      assert stats["oldest_event"] == "2026-07-01T00:00:00Z"
      assert stats["newest_event"] == "2026-07-03T00:00:00Z"
    end

    test "labels the result so a caller can tell it from Core's own numbers" do
      start_gateway(404)

      assert {:ok, stats} = CoreClient.get_stats()
      assert stats["approximate"] == true
      assert stats["source"] == "derived_from_events"
      assert stats["truncated"] == false
      assert stats["note"] =~ "HTTP 404"
    end

    test "also degrades on 405" do
      start_gateway(405)
      assert {:ok, %{"source" => "derived_from_events"}} = CoreClient.get_stats()
    end

    test "carries both keys, so quick_stats reads a real entity count" do
      start_gateway(404)
      {:ok, stats} = CoreClient.get_stats()

      # Core's StoreStats calls this total_entities; the MCP stats tools read
      # unique_entities. The derived payload provides both.
      assert stats["unique_entities"] == 2
      assert stats["total_entities"] == 2
    end
  end

  describe "get_stats/0 must not mask credential problems" do
    for status <- [401, 403] do
      test "#{status} surfaces as an error rather than an approximation" do
        start_gateway(unquote(status))

        assert {:error, message} = CoreClient.get_stats()
        assert message =~ "#{unquote(status)}"
        refute message =~ "derived_from_events"
      end
    end
  end

  describe "reconstruct_state/2 when the gateway does not route /entities/:id/state" do
    test "folds the entity's events, last write winning per key" do
      start_gateway(404)

      assert {:ok, state} = CoreClient.reconstruct_state("e1")

      # colour overwritten by the later event; size retained from the earlier one
      assert state["current_state"] == %{"colour" => "blue", "size" => 1}
      assert state["entity_id"] == "e1"
      assert state["event_count"] == 2
      assert state["last_updated"] == "2026-07-02T00:00:00Z"
      assert length(state["history"]) == 2
      assert state["source"] == "folded_from_events"
    end

    test "reports a missing entity rather than an empty state" do
      start_gateway(404)

      assert {:error, message} = CoreClient.reconstruct_state("does-not-exist")
      assert message =~ "Entity not found"
    end
  end

  describe "storage_stats/1" do
    test "inherits the stats fallback and marks compaction unavailable" do
      start_gateway(404)

      assert {:ok, result} = CoreClient.storage_stats()
      assert result["event_store"]["source"] == "derived_from_events"
      assert result["compaction"] == "unavailable"
    end
  end

  describe "exact path: gateway serves tenant-scoped stats (#230)" do
    # Once the gateway routes GET /api/v1/stats tenant-scoped, Core answers with
    # TenantStoreStats. This pins the cross-language shape contract: the field
    # names Rust serialises must be the ones the Elixir tools read. A mismatch
    # here is what made `unique_entities` render 0 against a real response.
    setup do
      original = Application.get_env(:mcp_server_elixir, :core_url)
      on_exit(fn -> Application.put_env(:mcp_server_elixir, :core_url, original) end)
      :ok
    end

    @tenant_stats %{
      "total_events" => 16_432,
      "total_entities" => 87,
      "total_event_types" => 12,
      "total_ingested" => 16_432,
      "event_types" => %{"created" => 9_001, "updated" => 7_431},
      "oldest_event" => "2026-01-02T03:04:05Z",
      "newest_event" => "2026-07-26T01:02:03Z"
    }

    test "returns Core's answer verbatim, with no fallback and no approximation" do
      start_stats_gateway(@tenant_stats)

      assert {:ok, stats} = CoreClient.get_stats()

      assert stats["total_events"] == 16_432
      assert stats["total_entities"] == 87
      assert stats["event_types"] == %{"created" => 9_001, "updated" => 7_431}

      # The derived-fallback markers must be absent — this is the exact path.
      refute Map.has_key?(stats, "source")
      refute Map.has_key?(stats, "approximate")
    end

    test "quick_stats renders the exact entity count from total_entities" do
      start_stats_gateway(@tenant_stats)

      state = %{
        backend: CoreClient,
        read_only: false,
        control_plane_enabled: false,
        system_admin: false
      }

      assert {:ok, %{content: [%{text: text} | _]}} =
               McpServerElixir.Protocol.McpTools.call_tool("quick_stats", %{}, state)

      assert text =~ "16432" or text =~ "16,432"
      # 87 entities must appear rather than the 0 the old key mismatch produced.
      assert text =~ "87"
    end
  end

  # Serves the given stats payload at /api/v1/stats (the routed, tenant-scoped
  # case) and events at /events/query.
  defp start_stats_gateway(stats) do
    {:ok, listen} =
      :gen_tcp.listen(0, [
        :binary,
        packet: :raw,
        active: false,
        reuseaddr: true,
        ip: {127, 0, 0, 1}
      ])

    {:ok, port} = :inet.port(listen)
    body = Jason.encode!(stats)

    spawn_link(fn -> stats_accept_loop(listen, body) end)
    Application.put_env(:mcp_server_elixir, :core_url, "http://127.0.0.1:#{port}")
    port
  end

  defp stats_accept_loop(listen, body) do
    case :gen_tcp.accept(listen) do
      {:ok, sock} ->
        spawn(fn ->
          with {:ok, data} <- :gen_tcp.recv(sock, 0, 5_000) do
            [line | _] = String.split(data, "\r\n")
            [_m, path | _] = String.split(line, " ")

            payload =
              if String.starts_with?(path, "/api/v1/stats"),
                do: body,
                else: Jason.encode!(%{"events" => [], "count" => 0})

            :gen_tcp.send(sock, [
              "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: #{byte_size(payload)}\r\n\r\n",
              payload
            ])
          end

          :gen_tcp.close(sock)
        end)

        stats_accept_loop(listen, body)

      _ ->
        :ok
    end
  end
end
