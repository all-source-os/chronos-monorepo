defmodule QueryServiceEx.Cluster.ConsistentHash do
  @moduledoc """
  Consistent hashing for entity-to-node mapping.

  Provides stable entity routing across the cluster:
  - Entities are consistently assigned to nodes
  - Node changes only affect a minimal subset of entities
  - Uses libring for hash ring implementation

  ## Usage

      # Get the node for an entity
      node = ConsistentHash.get_node("entity-123")

      # Get the node for a projection/entity combo
      node = ConsistentHash.get_node_for_projection("user_balance", "user-456")

      # Check if entity should be handled locally
      if ConsistentHash.local?("entity-123") do
        # Process locally
      else
        # Forward to appropriate node
      end

  ## How It Works

  The hash ring distributes entities across available nodes using consistent hashing.
  When a node joins or leaves:
  - Only entities mapped to that node are redistributed
  - Other entity-to-node mappings remain stable
  - This minimizes data movement during cluster changes
  """

  use GenServer

  require Logger

  @ring_name :query_service_hash_ring
  @virtual_nodes 128

  ## Client API

  @doc """
  Start the consistent hash ring manager.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Get the node responsible for a given key.

  ## Parameters
    * `key` - The key to hash (typically entity_id or combination)

  ## Returns
    * `node` - The node atom responsible for this key
    * `nil` - If no nodes are available
  """
  def get_node(key) when is_binary(key) do
    case :ets.lookup(@ring_name, :ring) do
      [{:ring, ring}] when ring != nil ->
        HashRing.key_to_node(ring, key)

      _ ->
        # No ring or empty ring - return local node
        node()
    end
  rescue
    ArgumentError -> node()
  end

  def get_node(key), do: get_node(to_string(key))

  @doc """
  Get the node responsible for a projection/entity combination.
  """
  def get_node_for_projection(projection_name, entity_id) do
    get_node("#{projection_name}:#{entity_id}")
  end

  @doc """
  Check if a key should be handled by the local node.
  """
  def local?(key) do
    get_node(key) == node()
  end

  @doc """
  Check if a projection/entity should be handled locally.
  """
  def local_for_projection?(projection_name, entity_id) do
    local?("#{projection_name}:#{entity_id}")
  end

  @doc """
  Get list of all nodes in the ring.
  """
  def nodes do
    case :ets.lookup(@ring_name, :nodes) do
      [{:nodes, nodes}] -> nodes
      _ -> [node()]
    end
  rescue
    ArgumentError -> [node()]
  end

  @doc """
  Get the number of nodes in the ring.
  """
  def node_count do
    length(nodes())
  end

  @doc """
  Force rebuild of the hash ring.
  """
  def rebuild do
    GenServer.call(__MODULE__, :rebuild)
  end

  @doc """
  Get keys that would be assigned to a specific node.

  This is useful for determining what data would migrate during node changes.
  Note: This is an approximation based on virtual node ownership.
  """
  def keys_for_node(target_node) do
    case :ets.lookup(@ring_name, :ring) do
      [{:ring, ring}] when ring != nil ->
        # Get all nodes and calculate percentage owned by target
        all_nodes = HashRing.nodes(ring)

        if target_node in all_nodes do
          # Return a representation of ownership
          %{
            node: target_node,
            total_nodes: length(all_nodes),
            approximate_share: 1.0 / length(all_nodes)
          }
        else
          %{node: target_node, total_nodes: 0, approximate_share: 0.0}
        end

      _ ->
        %{node: target_node, total_nodes: 0, approximate_share: 0.0}
    end
  end

  ## Server Callbacks

  @impl true
  def init(_opts) do
    # Create ETS table for storing the ring
    :ets.new(@ring_name, [:set, :public, :named_table, read_concurrency: true])

    # Subscribe to node membership events
    :net_kernel.monitor_nodes(true)

    # Build initial ring
    ring = build_ring()
    nodes = Node.list() ++ [node()]
    :ets.insert(@ring_name, {:ring, ring})
    :ets.insert(@ring_name, {:nodes, nodes})

    Logger.info("[ConsistentHash] Started with #{length(nodes)} node(s)")

    {:ok, %{ring: ring}}
  end

  @impl true
  def handle_call(:rebuild, _from, state) do
    ring = build_ring()
    nodes = Node.list() ++ [node()]
    :ets.insert(@ring_name, {:ring, ring})
    :ets.insert(@ring_name, {:nodes, nodes})

    Logger.info("[ConsistentHash] Ring rebuilt with #{length(nodes)} node(s)")

    {:reply, :ok, %{state | ring: ring}}
  end

  @impl true
  def handle_info({:nodeup, new_node}, state) do
    Logger.info("[ConsistentHash] Node joined: #{new_node}")

    ring = build_ring()
    nodes = Node.list() ++ [node()]
    :ets.insert(@ring_name, {:ring, ring})
    :ets.insert(@ring_name, {:nodes, nodes})

    # Broadcast node membership change
    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "cluster:membership",
      {:node_up, new_node, nodes}
    )

    {:noreply, %{state | ring: ring}}
  end

  @impl true
  def handle_info({:nodedown, dead_node}, state) do
    Logger.warning("[ConsistentHash] Node left: #{dead_node}")

    ring = build_ring()
    nodes = Node.list() ++ [node()]
    :ets.insert(@ring_name, {:ring, ring})
    :ets.insert(@ring_name, {:nodes, nodes})

    # Broadcast node membership change
    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "cluster:membership",
      {:node_down, dead_node, nodes}
    )

    {:noreply, %{state | ring: ring}}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  ## Private Functions

  defp build_ring do
    nodes = Node.list() ++ [node()]

    if Enum.empty?(nodes) do
      nil
    else
      Enum.reduce(nodes, HashRing.new(), fn node, ring ->
        HashRing.add_node(ring, node, @virtual_nodes)
      end)
    end
  end
end
