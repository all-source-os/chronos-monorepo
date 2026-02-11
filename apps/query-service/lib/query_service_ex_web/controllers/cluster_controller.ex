defmodule QueryServiceExWeb.ClusterController do
  @moduledoc """
  Controller for cluster health and management endpoints.

  Provides visibility into cluster state and distributed components.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Cluster.ConsistentHash
  alias QueryServiceEx.Cluster.DistributedRegistry
  alias QueryServiceEx.Cluster.DistributedSupervisor
  alias QueryServiceEx.Cluster.Membership
  alias QueryServiceEx.Cluster.Topology
  alias QueryServiceExWeb.Schemas.Cluster
  alias QueryServiceExWeb.Schemas.Common

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Cluster"])

  # -------------------------------------------------------------------
  # Cluster Health
  # -------------------------------------------------------------------

  operation(:health,
    summary: "Get cluster health",
    description: """
    Returns cluster health status including:
    - Node count and connectivity
    - Distributed registry status
    - Distributed supervisor status
    - Current node information
    """,
    responses: [
      ok: {"Cluster health", "application/json", Cluster.HealthResponse}
    ]
  )

  @doc """
  Get cluster health status.
  """
  def health(conn, _params) do
    health = Membership.health()

    json(conn, %{
      data: %{
        status: health.status,
        node: to_string(health.node),
        node_count: health.node_count,
        connected_peers: health.connected_peers,
        nodes: Enum.map(health.nodes, &to_string/1),
        registry_available: health.registry_available,
        supervisor_available: health.supervisor_available,
        uptime_seconds: health.uptime_seconds,
        issues: health.issues
      }
    })
  end

  # -------------------------------------------------------------------
  # Cluster Members
  # -------------------------------------------------------------------

  operation(:members,
    summary: "Get cluster members",
    description: "Returns list of all nodes in the cluster.",
    responses: [
      ok: {"Cluster members", "application/json", Cluster.MembersResponse}
    ]
  )

  @doc """
  Get list of cluster members.
  """
  def members(conn, _params) do
    nodes = Membership.nodes()

    members =
      Enum.map(nodes, fn node_name ->
        %{
          node: to_string(node_name),
          self: node_name == node(),
          connected: node_name == node() or node_name in Node.list()
        }
      end)

    json(conn, %{
      data: %{
        members: members,
        total: length(members),
        strategy: Topology.strategy()
      }
    })
  end

  # -------------------------------------------------------------------
  # Registry Info
  # -------------------------------------------------------------------

  operation(:registry,
    summary: "Get registry info",
    description: """
    Returns information about the distributed registry including:
    - Total registered processes
    - Registry members (nodes)
    - Availability status
    """,
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Registry info", "application/json", Cluster.RegistryResponse},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Get distributed registry information.
  """
  def registry(conn, _params) do
    available = DistributedRegistry.available?()

    data =
      if available do
        members = DistributedRegistry.members()
        count = DistributedRegistry.count()

        %{
          available: true,
          count: count,
          members: Enum.map(members, fn {node_name, _} -> to_string(node_name) end)
        }
      else
        %{
          available: false,
          count: 0,
          members: []
        }
      end

    json(conn, %{data: data})
  end

  # -------------------------------------------------------------------
  # Supervisor Info
  # -------------------------------------------------------------------

  operation(:supervisor,
    summary: "Get supervisor info",
    description: """
    Returns information about the distributed supervisor including:
    - Total child processes
    - Process distribution across nodes
    - Availability status
    """,
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Supervisor info", "application/json", Cluster.SupervisorResponse},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Get distributed supervisor information.
  """
  def supervisor(conn, _params) do
    available = DistributedSupervisor.available?()

    data =
      if available do
        children = DistributedSupervisor.which_children()
        counts = DistributedSupervisor.count_children()
        members = DistributedSupervisor.members()

        # Group children by node
        by_node =
          children
          |> Enum.group_by(fn {_id, pid, _type, _modules} ->
            if is_pid(pid), do: node(pid), else: :undefined
          end)
          |> Enum.map(fn {node_name, procs} -> {to_string(node_name), length(procs)} end)
          |> Map.new()

        %{
          available: true,
          total_children: counts[:workers] || 0,
          active: counts[:active] || 0,
          supervisors: counts[:supervisors] || 0,
          by_node: by_node,
          members: Enum.map(members, fn {node_name, _} -> to_string(node_name) end)
        }
      else
        %{
          available: false,
          total_children: 0,
          active: 0,
          supervisors: 0,
          by_node: %{},
          members: []
        }
      end

    json(conn, %{data: data})
  end

  # -------------------------------------------------------------------
  # Hash Ring Info
  # -------------------------------------------------------------------

  operation(:hash_ring,
    summary: "Get hash ring info",
    description: """
    Returns information about the consistent hash ring including:
    - Node distribution
    - Key routing information
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      key: [in: :query, type: :string, description: "Optional key to check routing"]
    ],
    responses: [
      ok: {"Hash ring info", "application/json", Cluster.HashRingResponse},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Get consistent hash ring information.
  """
  def hash_ring(conn, params) do
    nodes = ConsistentHash.nodes()

    data = %{
      node_count: length(nodes),
      nodes: Enum.map(nodes, &to_string/1),
      local_node: to_string(node())
    }

    # If a key is provided, show its routing
    data =
      case params["key"] do
        nil ->
          data

        key ->
          target_node = ConsistentHash.get_node(key)

          Map.put(data, :routing, %{
            key: key,
            routed_to: to_string(target_node),
            is_local: target_node == node()
          })
      end

    json(conn, %{data: data})
  end
end
