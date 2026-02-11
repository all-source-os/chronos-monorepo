defmodule QueryServiceExWeb.Schemas.Cluster do
  @moduledoc """
  OpenAPI schemas for cluster endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  # -------------------------------------------------------------------
  # Health Response
  # -------------------------------------------------------------------

  defmodule HealthData do
    @moduledoc "Cluster health data"
    OpenApiSpex.schema(%{
      title: "ClusterHealthData",
      description: "Cluster health status data",
      type: :object,
      properties: %{
        status: %Schema{
          type: :string,
          enum: ["healthy", "degraded", "leaving"],
          description: "Cluster health status"
        },
        node: %Schema{type: :string, description: "Current node name"},
        node_count: %Schema{type: :integer, description: "Total nodes in cluster"},
        connected_peers: %Schema{type: :integer, description: "Connected peer nodes"},
        nodes: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "List of all node names"
        },
        registry_available: %Schema{type: :boolean, description: "Distributed registry status"},
        supervisor_available: %Schema{
          type: :boolean,
          description: "Distributed supervisor status"
        },
        uptime_seconds: %Schema{type: :integer, description: "Node uptime in seconds"},
        issues: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "Current issues affecting health"
        }
      },
      example: %{
        status: "healthy",
        node: "query-service@10.0.1.5",
        node_count: 3,
        connected_peers: 2,
        nodes: ["query-service@10.0.1.5", "query-service@10.0.1.6", "query-service@10.0.1.7"],
        registry_available: true,
        supervisor_available: true,
        uptime_seconds: 3600,
        issues: []
      }
    })
  end

  defmodule HealthResponse do
    @moduledoc "Cluster health response"
    OpenApiSpex.schema(%{
      title: "ClusterHealthResponse",
      description: "Cluster health response",
      type: :object,
      properties: %{
        data: HealthData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Members Response
  # -------------------------------------------------------------------

  defmodule Member do
    @moduledoc "Cluster member"
    OpenApiSpex.schema(%{
      title: "ClusterMember",
      description: "Cluster member node",
      type: :object,
      properties: %{
        node: %Schema{type: :string, description: "Node name"},
        self: %Schema{type: :boolean, description: "Whether this is the current node"},
        connected: %Schema{type: :boolean, description: "Whether node is connected"}
      }
    })
  end

  defmodule MembersData do
    @moduledoc "Cluster members data"
    OpenApiSpex.schema(%{
      title: "ClusterMembersData",
      description: "Cluster members data",
      type: :object,
      properties: %{
        members: %Schema{
          type: :array,
          items: Member,
          description: "List of cluster members"
        },
        total: %Schema{type: :integer, description: "Total member count"},
        strategy: %Schema{type: :string, description: "Cluster discovery strategy"}
      }
    })
  end

  defmodule MembersResponse do
    @moduledoc "Members response"
    OpenApiSpex.schema(%{
      title: "ClusterMembersResponse",
      description: "Cluster members response",
      type: :object,
      properties: %{
        data: MembersData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Registry Response
  # -------------------------------------------------------------------

  defmodule RegistryData do
    @moduledoc "Registry data"
    OpenApiSpex.schema(%{
      title: "RegistryData",
      description: "Distributed registry data",
      type: :object,
      properties: %{
        available: %Schema{type: :boolean, description: "Registry availability"},
        count: %Schema{type: :integer, description: "Registered process count"},
        members: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "Registry member nodes"
        }
      }
    })
  end

  defmodule RegistryResponse do
    @moduledoc "Registry response"
    OpenApiSpex.schema(%{
      title: "RegistryResponse",
      description: "Distributed registry response",
      type: :object,
      properties: %{
        data: RegistryData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Supervisor Response
  # -------------------------------------------------------------------

  defmodule SupervisorData do
    @moduledoc "Supervisor data"
    OpenApiSpex.schema(%{
      title: "SupervisorData",
      description: "Distributed supervisor data",
      type: :object,
      properties: %{
        available: %Schema{type: :boolean, description: "Supervisor availability"},
        total_children: %Schema{type: :integer, description: "Total child processes"},
        active: %Schema{type: :integer, description: "Active children"},
        supervisors: %Schema{type: :integer, description: "Supervisor children"},
        by_node: %Schema{
          type: :object,
          additionalProperties: %Schema{type: :integer},
          description: "Process count per node"
        },
        members: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "Supervisor member nodes"
        }
      }
    })
  end

  defmodule SupervisorResponse do
    @moduledoc "Supervisor response"
    OpenApiSpex.schema(%{
      title: "SupervisorResponse",
      description: "Distributed supervisor response",
      type: :object,
      properties: %{
        data: SupervisorData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Hash Ring Response
  # -------------------------------------------------------------------

  defmodule KeyRouting do
    @moduledoc "Key routing info"
    OpenApiSpex.schema(%{
      title: "KeyRouting",
      description: "Key routing information",
      type: :object,
      properties: %{
        key: %Schema{type: :string, description: "The key being routed"},
        routed_to: %Schema{type: :string, description: "Target node"},
        is_local: %Schema{type: :boolean, description: "Whether target is local node"}
      }
    })
  end

  defmodule HashRingData do
    @moduledoc "Hash ring data"
    OpenApiSpex.schema(%{
      title: "HashRingData",
      description: "Consistent hash ring data",
      type: :object,
      properties: %{
        node_count: %Schema{type: :integer, description: "Nodes in ring"},
        nodes: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "Ring member nodes"
        },
        local_node: %Schema{type: :string, description: "Current node"},
        routing: KeyRouting
      }
    })
  end

  defmodule HashRingResponse do
    @moduledoc "Hash ring response"
    OpenApiSpex.schema(%{
      title: "HashRingResponse",
      description: "Hash ring response",
      type: :object,
      properties: %{
        data: HashRingData
      },
      required: [:data]
    })
  end
end
