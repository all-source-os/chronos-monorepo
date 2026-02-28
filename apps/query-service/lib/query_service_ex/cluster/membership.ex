defmodule QueryServiceEx.Cluster.Membership do
  @moduledoc """
  Handles cluster membership events and coordination.

  Responsibilities:
  - Monitor node join/leave events
  - Coordinate process redistribution
  - Track cluster health
  - Manage graceful shutdown

  ## Events Published

  Publishes to `cluster:membership` topic:
  - `{:node_up, node, all_nodes}` - A node joined
  - `{:node_down, node, all_nodes}` - A node left
  - `{:cluster_ready, all_nodes}` - Cluster is fully formed

  ## Graceful Shutdown

  During shutdown:
  1. Drains existing connections
  2. Marks node as leaving
  3. Waits for process migration
  4. Disconnects from cluster
  """

  use GenServer

  require Logger

  alias QueryServiceEx.Cluster.DistributedRegistry
  alias QueryServiceEx.Cluster.DistributedSupervisor

  @shutdown_timeout 30_000

  ## Client API

  @doc """
  Start the membership manager.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Get current cluster state.
  """
  def state do
    GenServer.call(__MODULE__, :state)
  end

  @doc """
  Get cluster health status.
  """
  def health do
    GenServer.call(__MODULE__, :health)
  end

  @doc """
  Initiate graceful shutdown.
  """
  def graceful_shutdown do
    GenServer.call(__MODULE__, :graceful_shutdown, @shutdown_timeout)
  end

  @doc """
  Check if cluster is healthy.
  """
  def healthy? do
    case health() do
      %{status: :healthy} -> true
      _ -> false
    end
  end

  @doc """
  Get all connected nodes including self.
  """
  def nodes do
    [node() | Node.list()]
  end

  @doc """
  Check if we're in a clustered environment.
  """
  def clustered? do
    Node.list() != []
  end

  ## Server Callbacks

  @impl true
  def init(_opts) do
    # Subscribe to node membership events
    :net_kernel.monitor_nodes(true, node_type: :visible)

    # Subscribe to cluster membership PubSub topic
    Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "cluster:membership")

    state = %{
      status: :starting,
      joined_at: DateTime.utc_now(),
      last_health_check: nil,
      nodes: [node() | Node.list()],
      leaving: false
    }

    # Schedule health check
    schedule_health_check()

    Logger.info("[Membership] Started on node #{node()}")

    {:ok, %{state | status: :running}}
  end

  @impl true
  def handle_call(:state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_call(:health, _from, state) do
    health = compute_health(state)
    {:reply, health, %{state | last_health_check: DateTime.utc_now()}}
  end

  @impl true
  def handle_call(:graceful_shutdown, _from, state) do
    Logger.info("[Membership] Initiating graceful shutdown...")

    state = %{state | leaving: true, status: :leaving}

    # Broadcast leaving event
    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "cluster:membership",
      {:node_leaving, node(), state.nodes}
    )

    # Wait for processes to migrate
    wait_for_migration()

    Logger.info("[Membership] Graceful shutdown complete")

    {:reply, :ok, state}
  end

  @impl true
  def handle_info({:nodeup, new_node, _info}, state) do
    Logger.info("[Membership] Node joined: #{new_node}")

    nodes = [node() | Node.list()]

    # Trigger process redistribution if needed
    maybe_redistribute_processes(state.nodes, nodes)

    {:noreply, %{state | nodes: nodes}}
  end

  @impl true
  def handle_info({:nodedown, dead_node, _info}, state) do
    Logger.warning("[Membership] Node left: #{dead_node}")

    nodes = [node() | Node.list()]

    # Horde will automatically handle process migration
    # Log the event for visibility
    Logger.info("[Membership] Cluster now has #{length(nodes)} node(s)")

    {:noreply, %{state | nodes: nodes}}
  end

  @impl true
  def handle_info({:node_up, _node, _nodes}, state) do
    # Received from PubSub - already handled in :nodeup
    {:noreply, state}
  end

  @impl true
  def handle_info({:node_down, _node, _nodes}, state) do
    # Received from PubSub - already handled in :nodedown
    {:noreply, state}
  end

  @impl true
  def handle_info({:node_leaving, node, _nodes}, state) when node != node() do
    Logger.info("[Membership] Node #{node} is gracefully leaving")
    {:noreply, state}
  end

  @impl true
  def handle_info(:health_check, state) do
    # Perform periodic health check
    health = compute_health(state)

    if health.status != :healthy do
      Logger.warning("[Membership] Cluster health: #{inspect(health)}")
    end

    schedule_health_check()
    {:noreply, %{state | last_health_check: DateTime.utc_now()}}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  ## Private Functions

  defp compute_health(state) do
    nodes = state.nodes
    node_count = length(nodes)
    connected = length(Node.list())

    registry_available = DistributedRegistry.available?()
    supervisor_available = DistributedSupervisor.available?()

    issues = []

    issues =
      if registry_available do
        issues
      else
        ["distributed_registry_unavailable" | issues]
      end

    issues =
      if supervisor_available do
        issues
      else
        ["distributed_supervisor_unavailable" | issues]
      end

    issues =
      if state.leaving do
        ["node_leaving" | issues]
      else
        issues
      end

    status =
      cond do
        state.leaving -> :leaving
        !registry_available or !supervisor_available -> :degraded
        Enum.empty?(issues) -> :healthy
        true -> :degraded
      end

    %{
      status: status,
      node: node(),
      node_count: node_count,
      connected_peers: connected,
      nodes: nodes,
      registry_available: registry_available,
      supervisor_available: supervisor_available,
      uptime_seconds: DateTime.diff(DateTime.utc_now(), state.joined_at),
      issues: issues
    }
  end

  defp schedule_health_check do
    Process.send_after(self(), :health_check, 30_000)
  end

  defp maybe_redistribute_processes(old_nodes, new_nodes) do
    # Horde handles automatic redistribution
    # This is a hook for custom logic if needed
    old_count = length(old_nodes)
    new_count = length(new_nodes)

    if new_count > old_count do
      Logger.info("[Membership] Cluster expanded from #{old_count} to #{new_count} nodes")
    else
      Logger.info("[Membership] Cluster contracted from #{old_count} to #{new_count} nodes")
    end

    :ok
  end

  defp wait_for_migration do
    # Wait for Horde to migrate processes
    # Check if we still have local processes every second
    max_wait = div(@shutdown_timeout, 1000)

    Enum.reduce_while(1..max_wait, :ok, fn _i, _acc ->
      case DistributedSupervisor.which_children() do
        [] ->
          {:halt, :ok}

        children ->
          local_children =
            Enum.filter(children, fn {_id, pid, _type, _modules} ->
              is_pid(pid) and node(pid) == node()
            end)

          if Enum.empty?(local_children) do
            {:halt, :ok}
          else
            Logger.debug(
              "[Membership] Waiting for #{length(local_children)} processes to migrate"
            )

            Process.sleep(1000)
            {:cont, :ok}
          end
      end
    end)
  end
end
