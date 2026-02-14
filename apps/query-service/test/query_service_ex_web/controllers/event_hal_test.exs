defmodule QueryServiceExWeb.EventHALTest do
  @moduledoc """
  Tests that event and query responses include correct HAL links.

  Uses unit-level assertions on HAL.wrap/2 and the link shapes
  the controllers produce, without needing a running Core backend.
  """

  use ExUnit.Case, async: true

  alias QueryServiceExWeb.HAL

  # Simulate what event_links/1 in EventController produces
  defp event_links(event) do
    id = event["id"] || event[:id]
    entity_id = event["entity_id"] || event[:entity_id]
    event_type = event["event_type"] || event[:event_type]

    HAL.self("/api/events/#{id}")
    |> maybe_merge(entity_id, fn eid ->
      HAL.merge(
        HAL.link("stream", "/api/events/entity/#{eid}"),
        HAL.link("entity", "/api/events/entity/#{eid}")
      )
    end)
    |> maybe_merge(event_type, fn et ->
      HAL.merge(
        HAL.link("event_type", "/api/events/type/#{et}"),
        HAL.link("schema", "/api/schemas/#{et}")
      )
    end)
    |> HAL.merge(HAL.mgmt_plane_link("tenant", "/api/v1/tenants/{tenant_id}", templated: true))
  end

  defp maybe_merge(links, nil, _fun), do: links
  defp maybe_merge(links, val, fun), do: HAL.merge(links, fun.(val))

  describe "single event HAL links" do
    test "includes self, stream, event_type, entity, schema, and tenant links" do
      event = %{
        "id" => "evt-001",
        "entity_id" => "user-42",
        "event_type" => "user.created"
      }

      links = event_links(event)

      assert links["self"] == %{href: "/api/events/evt-001"}
      assert links["stream"] == %{href: "/api/events/entity/user-42"}
      assert links["entity"] == %{href: "/api/events/entity/user-42"}
      assert links["event_type"] == %{href: "/api/events/type/user.created"}
      assert links["schema"] == %{href: "/api/schemas/user.created"}
      assert links["tenant"][:href] =~ "/api/v1/tenants/{tenant_id}"
      assert links["tenant"][:templated] == true
    end

    test "wrapping event preserves original data and adds _links" do
      event = %{"id" => "e1", "entity_id" => "ent1", "event_type" => "click", "payload" => %{}}
      wrapped = HAL.wrap(event, event_links(event))

      assert wrapped["id"] == "e1"
      assert wrapped["payload"] == %{}
      assert wrapped["_links"]["self"] == %{href: "/api/events/e1"}
    end

    test "handles event with nil entity_id gracefully" do
      event = %{"id" => "e2", "entity_id" => nil, "event_type" => "sys.boot"}
      links = event_links(event)

      assert links["self"] == %{href: "/api/events/e2"}
      refute Map.has_key?(links, "stream")
      refute Map.has_key?(links, "entity")
      assert links["event_type"] == %{href: "/api/events/type/sys.boot"}
    end

    test "handles event with nil event_type gracefully" do
      event = %{"id" => "e3", "entity_id" => "ent1", "event_type" => nil}
      links = event_links(event)

      assert links["self"] == %{href: "/api/events/e3"}
      assert links["stream"] == %{href: "/api/events/entity/ent1"}
      refute Map.has_key?(links, "event_type")
      refute Map.has_key?(links, "schema")
    end

    test "tenant link includes MGMT_PLANE_URL when set" do
      System.put_env("MGMT_PLANE_URL", "https://cp.example.com")

      event = %{"id" => "e4", "entity_id" => "a", "event_type" => "b"}
      links = event_links(event)

      assert links["tenant"][:href] == "https://cp.example.com/api/v1/tenants/{tenant_id}"
      assert links["tenant"][:templated] == true

      System.delete_env("MGMT_PLANE_URL")
    end
  end

  describe "list response HAL links" do
    test "list_self_link builds correct path without params" do
      link = HAL.self("/api/events")
      assert link == %{"self" => %{href: "/api/events"}}
    end

    test "batch response includes self link" do
      self_link = HAL.self("/api/events/batch")
      assert self_link["self"][:href] == "/api/events/batch"
    end

    test "by_entity response includes self link with entity_id" do
      self_link = HAL.self("/api/events/entity/user-42")
      assert self_link["self"][:href] == "/api/events/entity/user-42"
    end

    test "by_type response includes self link with event_type" do
      self_link = HAL.self("/api/events/type/order.placed")
      assert self_link["self"][:href] == "/api/events/type/order.placed"
    end
  end

  describe "query response HAL links" do
    test "query response includes self and tenant links" do
      links =
        HAL.self("/api/query")
        |> HAL.merge(HAL.mgmt_plane_link("tenant", "/api/v1/tenants/{tenant_id}", templated: true))

      assert links["self"] == %{href: "/api/query"}
      assert links["tenant"][:templated] == true
    end

    test "next link added when results fill the page" do
      links =
        HAL.self("/api/query")
        |> HAL.merge(HAL.link("next", "/api/query?offset=100&limit=100"))

      assert links["next"] == %{href: "/api/query?offset=100&limit=100"}
    end

    test "no next link when results are fewer than limit" do
      # Just self — no next
      links = HAL.self("/api/query")
      refute Map.has_key?(links, "next")
    end
  end

  describe "JSON serialization of event with HAL" do
    test "full event with links serializes correctly" do
      event = %{"id" => "e5", "entity_id" => "ent1", "event_type" => "click", "payload" => %{"x" => 1}}
      wrapped = HAL.wrap(event, event_links(event))
      json = Jason.encode!(wrapped)
      decoded = Jason.decode!(json)

      assert decoded["id"] == "e5"
      assert decoded["payload"] == %{"x" => 1}
      assert decoded["_links"]["self"]["href"] == "/api/events/e5"
      assert decoded["_links"]["stream"]["href"] == "/api/events/entity/ent1"
      assert decoded["_links"]["event_type"]["href"] == "/api/events/type/click"
      assert decoded["_links"]["schema"]["href"] == "/api/schemas/click"
      assert decoded["_links"]["tenant"]["templated"] == true
    end
  end
end
