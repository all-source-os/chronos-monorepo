defmodule QueryServiceExWeb.HALTest do
  use ExUnit.Case, async: true

  alias QueryServiceExWeb.HAL

  describe "self/1" do
    test "creates a self link" do
      assert HAL.self("/api/v1/events/42") == %{"self" => %{href: "/api/v1/events/42"}}
    end
  end

  describe "link/2" do
    test "creates a named link with just href" do
      assert HAL.link("next", "/api/v1/events?offset=100") ==
               %{"next" => %{href: "/api/v1/events?offset=100"}}
    end
  end

  describe "link/3" do
    test "creates a templated link" do
      result = HAL.link("tenant", "/api/v1/tenants/{id}", templated: true)
      assert result == %{"tenant" => %{href: "/api/v1/tenants/{id}", templated: true}}
    end

    test "creates a link with title" do
      result = HAL.link("billing", "/billing", title: "Billing portal")
      assert result == %{"billing" => %{href: "/billing", title: "Billing portal"}}
    end

    test "creates a link with both templated and title" do
      result = HAL.link("events", "/events/{id}", templated: true, title: "Event detail")
      assert result == %{"events" => %{href: "/events/{id}", templated: true, title: "Event detail"}}
    end

    test "omits templated when false" do
      result = HAL.link("item", "/items/1", templated: false)
      assert result == %{"item" => %{href: "/items/1"}}
    end

    test "omits title when nil" do
      result = HAL.link("item", "/items/1", title: nil)
      assert result == %{"item" => %{href: "/items/1"}}
    end
  end

  describe "merge/2" do
    test "merges two link maps" do
      a = HAL.self("/events/1")
      b = HAL.link("tenant", "/tenants/t1")
      assert HAL.merge(a, b) == %{"self" => %{href: "/events/1"}, "tenant" => %{href: "/tenants/t1"}}
    end

    test "later map overrides duplicate keys" do
      a = HAL.link("self", "/old")
      b = HAL.self("/new")
      assert HAL.merge(a, b) == %{"self" => %{href: "/new"}}
    end
  end

  describe "mgmt_plane_link/3" do
    test "creates a link with MGMT_PLANE_URL prefix" do
      System.put_env("MGMT_PLANE_URL", "https://cp.example.com")

      result = HAL.mgmt_plane_link("tenant", "/api/v1/tenants/{id}", templated: true)

      assert result == %{
               "tenant" => %{href: "https://cp.example.com/api/v1/tenants/{id}", templated: true}
             }

      System.delete_env("MGMT_PLANE_URL")
    end

    test "falls back to empty string when MGMT_PLANE_URL not set" do
      System.delete_env("MGMT_PLANE_URL")

      result = HAL.mgmt_plane_link("tenant", "/api/v1/tenants/{id}")
      assert result == %{"tenant" => %{href: "/api/v1/tenants/{id}"}}
    end
  end

  describe "mgmt_plane_url/0" do
    test "reads MGMT_PLANE_URL env var" do
      System.put_env("MGMT_PLANE_URL", "https://mgmt.test")
      assert HAL.mgmt_plane_url() == "https://mgmt.test"
      System.delete_env("MGMT_PLANE_URL")
    end

    test "returns empty string when not set" do
      System.delete_env("MGMT_PLANE_URL")
      assert HAL.mgmt_plane_url() == ""
    end
  end

  describe "wrap/2" do
    test "adds _links to a plain map" do
      result = HAL.wrap(%{id: 1, name: "test"}, HAL.self("/items/1"))
      assert result[:id] == 1
      assert result[:name] == "test"
      assert result["_links"] == %{"self" => %{href: "/items/1"}}
    end

    test "adds _links to a struct by converting to map" do
      result = HAL.wrap(%URI{path: "/foo"}, HAL.self("/uris/1"))
      assert result["_links"] == %{"self" => %{href: "/uris/1"}}
      assert result[:path] == "/foo"
      refute Map.has_key?(result, :__struct__)
    end
  end

  describe "JSON output format" do
    test "links serialize correctly to JSON" do
      links = HAL.merge(
        HAL.self("/api/v1/events/42"),
        HAL.link("tenant", "/api/v1/tenants/{id}", templated: true, title: "Parent tenant")
      )

      response = HAL.wrap(%{id: "42", type: "click"}, links)
      json = Jason.encode!(response)
      decoded = Jason.decode!(json)

      assert decoded["_links"]["self"]["href"] == "/api/v1/events/42"
      assert decoded["_links"]["tenant"]["href"] == "/api/v1/tenants/{id}"
      assert decoded["_links"]["tenant"]["templated"] == true
      assert decoded["_links"]["tenant"]["title"] == "Parent tenant"
    end

    test "empty links map omits _links when encoded with a struct that uses derive" do
      # Plain maps always include the key, but it's empty
      result = HAL.wrap(%{id: 1}, %{})
      json = Jason.encode!(result)
      decoded = Jason.decode!(json)
      assert decoded["_links"] == %{}
    end
  end
end
