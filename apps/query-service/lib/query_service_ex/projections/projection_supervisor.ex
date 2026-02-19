defmodule QueryServiceEx.Projections.ProjectionSupervisor do
  @moduledoc """
  DynamicSupervisor that starts a ProjectionServer for each projection
  registered in the Registry. Starts on QS application boot.

  Provides `start_projection/1` and `stop_projection/1` for dynamic
  management of individual projection servers.
  """

  use DynamicSupervisor
  require Logger

  alias QueryServiceEx.Projections.ProjectionServer
  alias QueryServiceEx.Projections.Registry

  @doc "Starts the ProjectionSupervisor and boots all registered projections."
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)

    case DynamicSupervisor.start_link(__MODULE__, opts, name: name) do
      {:ok, pid} ->
        start_all(name)
        {:ok, pid}

      error ->
        error
    end
  end

  @impl true
  def init(_opts) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end

  @doc """
  Called after the supervisor is started to boot projection servers
  for all registered projections.
  """
  def start_all(supervisor \\ __MODULE__) do
    for name <- Registry.list() do
      case start_projection(name, supervisor) do
        {:ok, _pid} ->
          Logger.info("ProjectionSupervisor started projection: #{name}")
          :ok

        {:error, reason} ->
          Logger.warning(
            "ProjectionSupervisor failed to start projection #{name}: #{inspect(reason)}"
          )

          :error
      end
    end
  end

  @doc """
  Starts a ProjectionServer for the given projection name.

  Returns `{:ok, pid}` or `{:error, reason}`.
  """
  def start_projection(projection_name, supervisor \\ __MODULE__) do
    case Registry.get(projection_name) do
      nil ->
        {:error, :unknown_projection}

      projection_mod ->
        child_spec = {
          ProjectionServer,
          [
            projection: projection_mod,
            name: via_name(projection_name)
          ]
        }

        DynamicSupervisor.start_child(supervisor, child_spec)
    end
  end

  @doc """
  Stops the ProjectionServer for the given projection name.

  Returns `:ok` or `{:error, :not_found}`.
  """
  def stop_projection(projection_name, supervisor \\ __MODULE__) do
    case GenServer.whereis(via_name(projection_name)) do
      nil ->
        {:error, :not_found}

      pid ->
        DynamicSupervisor.terminate_child(supervisor, pid)
    end
  end

  defp via_name(projection_name) do
    {:via, Elixir.Registry,
     {QueryServiceEx.ProjectionRegistry, {:projection_server, projection_name}}}
  end
end
