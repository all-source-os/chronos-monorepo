defmodule QueryServiceEx.Cluster.DistributedRegistry do
  @moduledoc """
  Distributed registry for projection processes using Horde.

  Provides CRDT-based registry that works across cluster nodes:
  - Automatic process migration on node failure
  - Consistent process location across the cluster
  - Integration with projection sync processes

  ## Usage

      # Register a process
      DistributedRegistry.register({:projection, "my_projection", "entity_123"}, self())

      # Lookup a process
      case DistributedRegistry.whereis({:projection, "my_projection", "entity_123"}) do
        :undefined -> # Start process
        pid -> # Use existing process
      end

      # Get all registered names
      DistributedRegistry.registered_names()
  """

  require Logger

  @doc """
  Start the distributed registry.
  """
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)

    Horde.Registry.start_link(
      name: name,
      keys: :unique,
      members: :auto
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
  Register the calling process under a name.

  ## Parameters
    * `name` - The name to register (usually a tuple like `{:projection, name, entity_id}`)
    * `pid` - The process to register (defaults to self())

  ## Returns
    * `{:ok, pid}` - Successfully registered
    * `{:error, {:already_registered, pid}}` - Name already taken
  """
  def register(name, pid \\ self()) do
    Horde.Registry.register(__MODULE__, name, pid)
  end

  @doc """
  Unregister a name.
  """
  def unregister(name) do
    Horde.Registry.unregister(__MODULE__, name)
  end

  @doc """
  Lookup a process by name.

  ## Returns
    * `pid` - The registered process
    * `:undefined` - No process registered under that name
  """
  def whereis(name) do
    case Horde.Registry.lookup(__MODULE__, name) do
      [{pid, _value}] -> pid
      [] -> :undefined
    end
  end

  @doc """
  Lookup a process by name, returning full tuple.

  ## Returns
    * `{pid, value}` - The registered process and its value
    * `nil` - No process registered under that name
  """
  def lookup(name) do
    case Horde.Registry.lookup(__MODULE__, name) do
      [{pid, value}] -> {pid, value}
      [] -> nil
    end
  end

  @doc """
  Get all registered names.
  """
  def registered_names do
    # Horde.Registry doesn't have a direct "list all" function
    # We need to use select with a match-all pattern
    Horde.Registry.select(__MODULE__, [{{:"$1", :"$2", :"$3"}, [], [{{:"$1", :"$2"}}]}])
    |> Enum.map(fn {name, _pid} -> name end)
  end

  @doc """
  Count registered processes.
  """
  def count do
    Horde.Registry.count(__MODULE__)
  end

  @doc """
  Get all registered processes for a specific projection.
  """
  def list_projection_processes(projection_name) do
    Horde.Registry.select(__MODULE__, [
      {{{:projection, projection_name, :"$1"}, :"$2", :"$3"}, [], [{{:"$1", :"$2"}}]}
    ])
  end

  @doc """
  Get registry members (nodes participating in the registry).

  Returns a list of {node, pid} tuples for all nodes in the registry.
  """
  def members do
    # Get the list of nodes that are members of the registry
    # In Horde, this is managed internally via CRDT membership
    [node() | Node.list()]
    |> Enum.map(fn n -> {n, nil} end)
  end

  @doc """
  Check if the registry is available.
  """
  def available? do
    case Process.whereis(__MODULE__) do
      nil -> false
      _pid -> true
    end
  end
end
