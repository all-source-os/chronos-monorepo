defmodule QueryServiceExWeb.HAL do
  @moduledoc """
  Reusable HAL (Hypertext Application Language) link builder.

  Provides functions for building standardized `_links` maps
  that conform to the HAL specification (RFC draft-kelly-json-hal).

  ## Usage

      iex> HAL.self("/api/v1/events/123")
      %{"self" => %{href: "/api/v1/events/123"}}

      iex> HAL.link("tenant", "/api/v1/tenants/{tenant_id}", templated: true, title: "Tenant")
      %{"tenant" => %{href: "/api/v1/tenants/{tenant_id}", templated: true, title: "Tenant"}}
  """

  @type link :: %{
          required(:href) => String.t(),
          optional(:templated) => boolean(),
          optional(:title) => String.t()
        }
  @type links :: %{optional(String.t()) => link()}

  @doc """
  Creates a `self` link for the given path.

  Returns a map with a single `"self"` key.

  ## Examples

      iex> QueryServiceExWeb.HAL.self("/api/v1/events/42")
      %{"self" => %{href: "/api/v1/events/42"}}
  """
  @spec self(String.t()) :: links()
  def self(path) when is_binary(path) do
    %{"self" => %{href: path}}
  end

  @doc """
  Creates a named HAL link.

  ## Options

    * `:templated` - marks the link as a URI template (default: `false`)
    * `:title` - human-readable title for the link

  ## Examples

      iex> QueryServiceExWeb.HAL.link("next", "/api/v1/events?offset=100")
      %{"next" => %{href: "/api/v1/events?offset=100"}}

      iex> QueryServiceExWeb.HAL.link("tenant", "/api/v1/tenants/{id}", templated: true)
      %{"tenant" => %{href: "/api/v1/tenants/{id}", templated: true}}

      iex> QueryServiceExWeb.HAL.link("billing", "/billing", title: "Billing portal")
      %{"billing" => %{href: "/billing", title: "Billing portal"}}
  """
  @spec link(String.t(), String.t(), keyword()) :: links()
  def link(rel, href, opts \\ []) when is_binary(rel) and is_binary(href) do
    link_obj =
      %{href: href}
      |> maybe_put(:templated, opts[:templated])
      |> maybe_put(:title, opts[:title])

    %{rel => link_obj}
  end

  @doc """
  Merges multiple link maps into one. Later maps override earlier ones for duplicate keys.

  ## Examples

      iex> QueryServiceExWeb.HAL.merge(
      ...>   QueryServiceExWeb.HAL.self("/events/1"),
      ...>   QueryServiceExWeb.HAL.link("tenant", "/tenants/t1")
      ...> )
      %{"self" => %{href: "/events/1"}, "tenant" => %{href: "/tenants/t1"}}
  """
  @spec merge(links(), links()) :: links()
  def merge(a, b) when is_map(a) and is_map(b), do: Map.merge(a, b)

  @doc """
  Creates a link pointing to the management plane (Control Plane).

  Reads `MGMT_PLANE_URL` env var and prepends it to the path.
  Falls back to empty string if not set.

  ## Examples

      iex> QueryServiceExWeb.HAL.mgmt_plane_link("tenant", "/api/v1/tenants/{id}", templated: true)
      %{"tenant" => %{href: "/api/v1/tenants/{id}", templated: true}}
  """
  @spec mgmt_plane_link(String.t(), String.t(), keyword()) :: links()
  def mgmt_plane_link(rel, path, opts \\ []) do
    base = mgmt_plane_url()
    link(rel, base <> path, opts)
  end

  @doc """
  Returns the configured `MGMT_PLANE_URL` for cross-service links to the Control Plane.

  Returns empty string if not set.
  """
  @spec mgmt_plane_url() :: String.t()
  def mgmt_plane_url do
    System.get_env("MGMT_PLANE_URL") || ""
  end

  @doc """
  Wraps a map/struct with HAL `_links`.

  Adds an `"_links"` key to the given map. If the input is a struct,
  it is first converted to a plain map (dropping the `__struct__` key).

  ## Examples

      iex> QueryServiceExWeb.HAL.wrap(%{id: 1, name: "test"}, QueryServiceExWeb.HAL.self("/items/1"))
      %{id: 1, name: "test", "_links" => %{"self" => %{href: "/items/1"}}}
  """
  @spec wrap(map(), links()) :: map()
  def wrap(%_{} = struct, links), do: struct |> Map.from_struct() |> Map.put("_links", links)
  def wrap(map, links) when is_map(map), do: Map.put(map, "_links", links)

  # Only puts the key if value is truthy (non-nil, non-false).
  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, _key, false), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
end
