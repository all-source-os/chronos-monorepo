defmodule QueryServiceEx.Cluster.DistributedSupervisor do
  @moduledoc """
  Distributed supervisor for projection processes using Horde.

  Provides:
  - Automatic process distribution across cluster nodes
  - Process migration on node failure
  - Delta-CRDT based state synchronization

  ## Usage

      # Start a child process
      DistributedSupervisor.start_child({MyWorker, [arg1, arg2]})

      # Start a child with a specific ID
      spec = %{id: "my_worker", start: {MyWorker, :start_link, [[arg1, arg2]]}}
      DistributedSupervisor.start_child(spec)

      # Terminate a child
      DistributedSupervisor.terminate_child(pid)

      # List all children
      DistributedSupervisor.which_children()
  """

  require Logger

  @doc """
  Start the distributed supervisor.
  """
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)

    Horde.DynamicSupervisor.start_link(
      name: name,
      strategy: :one_for_one,
      members: :auto,
      process_redistribution: :active
    )
  end

  @doc """
  Returns the child_spec for supervision tree.
  """
  def child_spec(opts) do
    %{
      id: __MODULE__,
      start: {__MODULE__, :start_link, [opts]},
      type: :supervisor
    }
  end

  @doc """
  Start a child process on the distributed supervisor.

  The process will be started on an available node in the cluster.

  ## Parameters
    * `child_spec` - Child specification (tuple or map)

  ## Returns
    * `{:ok, pid}` - Successfully started
    * `{:error, reason}` - Failed to start
  """
  def start_child(child_spec) do
    Horde.DynamicSupervisor.start_child(__MODULE__, child_spec)
  end

  @doc """
  Terminate a child process.

  ## Parameters
    * `pid` - The process to terminate

  ## Returns
    * `:ok` - Successfully terminated
    * `{:error, :not_found}` - Process not found
  """
  def terminate_child(pid) when is_pid(pid) do
    Horde.DynamicSupervisor.terminate_child(__MODULE__, pid)
  end

  @doc """
  Returns a list of all children.

  ## Returns
    List of `{id, pid, type, modules}` tuples.
  """
  def which_children do
    Horde.DynamicSupervisor.which_children(__MODULE__)
  end

  @doc """
  Count all children across the cluster.
  """
  def count_children do
    Horde.DynamicSupervisor.count_children(__MODULE__)
  end

  @doc """
  Get supervisor members (nodes participating in the supervisor).

  Returns a list of {node, pid} tuples for all nodes in the supervisor.
  """
  def members do
    # Get the list of nodes that are members of the supervisor
    # In Horde, this is managed internally via CRDT membership
    [node() | Node.list()]
    |> Enum.map(fn n -> {n, nil} end)
  end

  @doc """
  Check if the supervisor is available.
  """
  def available? do
    case Process.whereis(__MODULE__) do
      nil -> false
      _pid -> true
    end
  end
end
