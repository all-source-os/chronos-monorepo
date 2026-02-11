defmodule QueryServiceEx.Cluster.Topology do
  @moduledoc """
  Cluster topology configuration for automatic node discovery.

  Supports multiple strategies:
  - **Kubernetes** - Uses Kubernetes API for service discovery
  - **DNS** - Uses DNS polling for node discovery
  - **Gossip** - Uses UDP multicast for local network discovery
  - **Epmd** - Uses Erlang Port Mapper Daemon (development)

  ## Configuration

  Set `CLUSTER_STRATEGY` environment variable to one of:
  - `kubernetes` - For Kubernetes deployments
  - `dns` - For DNS-based discovery
  - `gossip` - For local network discovery
  - `epmd` - For development/static node list
  - `none` - Disable clustering (default)

  ### Kubernetes Strategy

      CLUSTER_STRATEGY=kubernetes
      CLUSTER_KUBERNETES_NAMESPACE=default
      CLUSTER_KUBERNETES_SELECTOR=app=query-service
      CLUSTER_KUBERNETES_NODE_BASENAME=query-service

  ### DNS Strategy

      CLUSTER_STRATEGY=dns
      CLUSTER_DNS_QUERY=query-service.default.svc.cluster.local
      CLUSTER_DNS_NODE_BASENAME=query-service

  ### Gossip Strategy

      CLUSTER_STRATEGY=gossip
      CLUSTER_GOSSIP_PORT=45892
      CLUSTER_GOSSIP_SECRET=shared_secret

  ## Topology Types

  The topology is a child_spec that can be added to the supervision tree.
  """

  require Logger

  @doc """
  Returns the cluster topology configuration for libcluster.

  Returns `nil` if clustering is disabled.
  """
  def child_spec do
    strategy = System.get_env("CLUSTER_STRATEGY", "none")

    case strategy do
      "none" ->
        Logger.info("[Cluster] Clustering disabled (CLUSTER_STRATEGY=none)")
        nil

      "kubernetes" ->
        Logger.info("[Cluster] Using Kubernetes strategy")
        kubernetes_topology()

      "dns" ->
        Logger.info("[Cluster] Using DNS strategy")
        dns_topology()

      "gossip" ->
        Logger.info("[Cluster] Using Gossip strategy")
        gossip_topology()

      "epmd" ->
        Logger.info("[Cluster] Using EPMD strategy")
        epmd_topology()

      unknown ->
        Logger.warning("[Cluster] Unknown strategy '#{unknown}', clustering disabled")
        nil
    end
  end

  @doc """
  Returns configured cluster strategy name.
  """
  def strategy do
    System.get_env("CLUSTER_STRATEGY", "none")
  end

  @doc """
  Returns list of connected nodes excluding self.
  """
  def connected_nodes do
    Node.list()
  end

  @doc """
  Returns total node count including self.
  """
  def node_count do
    length(Node.list()) + 1
  end

  @doc """
  Check if clustering is enabled.
  """
  def enabled? do
    strategy() != "none"
  end

  # Private topology builders

  defp kubernetes_topology do
    namespace = System.get_env("CLUSTER_KUBERNETES_NAMESPACE", "default")
    selector = System.get_env("CLUSTER_KUBERNETES_SELECTOR", "app=query-service")
    node_basename = System.get_env("CLUSTER_KUBERNETES_NODE_BASENAME", "query-service")
    polling_interval = String.to_integer(System.get_env("CLUSTER_POLLING_INTERVAL", "5000"))

    {Cluster.Supervisor,
     [
       name: QueryServiceEx.ClusterSupervisor,
       topologies: [
         k8s: [
           strategy: Cluster.Strategy.Kubernetes,
           config: [
             mode: :dns,
             kubernetes_ip_lookup_mode: :pods,
             kubernetes_namespace: namespace,
             kubernetes_selector: selector,
             kubernetes_node_basename: node_basename,
             polling_interval: polling_interval
           ]
         ]
       ]
     ]}
  end

  defp dns_topology do
    query = System.get_env("CLUSTER_DNS_QUERY", "query-service.default.svc.cluster.local")
    node_basename = System.get_env("CLUSTER_DNS_NODE_BASENAME", "query-service")
    polling_interval = String.to_integer(System.get_env("CLUSTER_POLLING_INTERVAL", "5000"))

    {Cluster.Supervisor,
     [
       name: QueryServiceEx.ClusterSupervisor,
       topologies: [
         dns: [
           strategy: Cluster.Strategy.DNSPoll,
           config: [
             polling_interval: polling_interval,
             query: query,
             node_basename: node_basename
           ]
         ]
       ]
     ]}
  end

  defp gossip_topology do
    port = String.to_integer(System.get_env("CLUSTER_GOSSIP_PORT", "45892"))
    secret = System.get_env("CLUSTER_GOSSIP_SECRET", "chronos_cluster_secret")

    {Cluster.Supervisor,
     [
       name: QueryServiceEx.ClusterSupervisor,
       topologies: [
         gossip: [
           strategy: Cluster.Strategy.Gossip,
           config: [
             port: port,
             if_addr: "0.0.0.0",
             multicast_if: "0.0.0.0",
             multicast_addr: "230.1.1.251",
             multicast_ttl: 1,
             secret: secret
           ]
         ]
       ]
     ]}
  end

  defp epmd_topology do
    # For development: comma-separated list of nodes
    hosts_str = System.get_env("CLUSTER_EPMD_HOSTS", "")

    hosts =
      hosts_str
      |> String.split(",", trim: true)
      |> Enum.map(&String.trim/1)
      |> Enum.map(&String.to_atom/1)

    if Enum.empty?(hosts) do
      Logger.warning("[Cluster] EPMD strategy configured but no hosts specified")
      nil
    else
      {Cluster.Supervisor,
       [
         name: QueryServiceEx.ClusterSupervisor,
         topologies: [
           epmd: [
             strategy: Cluster.Strategy.Epmd,
             config: [
               hosts: hosts
             ]
           ]
         ]
       ]}
    end
  end
end
