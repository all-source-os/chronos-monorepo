defmodule QueryServiceEx.Domain.Entities.Projection do
  @moduledoc """
  Domain entity for projections.

  A projection transforms event streams into queryable read models.
  This module provides pure domain logic for projection definitions and state management.
  """

  defmodule Definition do
    @moduledoc "Projection definition entity"
    @type project_fn :: (any(), map() -> any())

    @type t :: %__MODULE__{
            name: atom(),
            version: pos_integer(),
            initial_state: any(),
            project_fn: project_fn(),
            metadata: map()
          }

    @enforce_keys [:name, :version, :initial_state, :project_fn]
    defstruct [:name, :version, :initial_state, :project_fn, metadata: %{}]

    @doc """
    Create a new projection definition.

    ## Parameters
      * `name` - Atom identifier for the projection
      * `version` - Integer version number (for migration tracking)
      * `initial_state` - Initial state value (any data structure)
      * `project_fn` - Function `(state, event) -> new_state`
      * `metadata` - Optional metadata map

    ## Examples

        iex> Definition.new(
        ...>   name: :user_statistics,
        ...>   version: 1,
        ...>   initial_state: %{count: 0, total_spent: 0.0},
        ...>   project_fn: fn state, event ->
        ...>     case event.event_type do
        ...>       "order.placed" ->
        ...>         state
        ...>         |> Map.update!(:count, &(&1 + 1))
        ...>         |> Map.update!(:total_spent, &(&1 + event.payload.amount))
        ...>       _ -> state
        ...>     end
        ...>   end
        ...> )
    """
    def new(opts) do
      struct!(__MODULE__, opts)
    end
  end

  defmodule Snapshot do
    @moduledoc "Projection state snapshot entity"
    @type t :: %__MODULE__{
            projection_name: atom(),
            entity_id: String.t(),
            state: any(),
            version: pos_integer(),
            timestamp: DateTime.t()
          }

    @enforce_keys [:projection_name, :entity_id, :state, :version, :timestamp]
    defstruct [:projection_name, :entity_id, :state, :version, :timestamp]

    @doc "Create a new projection snapshot"
    def new(opts) do
      struct!(__MODULE__, opts)
    end
  end

  ## Validation Functions

  @doc "Check if version is valid (positive integer)"
  def valid_version?(version) when is_integer(version) and version > 0, do: true
  def valid_version?(_), do: false

  @doc "Check if projection name is valid (must be atom)"
  def valid_name?(name) when is_atom(name), do: true
  def valid_name?(_), do: false

  @doc "Validate a complete projection definition"
  def valid_projection?(%Definition{name: name, version: version, initial_state: initial, project_fn: project_fn})
      when is_function(project_fn, 2) do
    valid_name?(name) and valid_version?(version) and not is_nil(initial)
  end

  def valid_projection?(_), do: false

  ## Projection Application (Pure Functions)

  @doc """
  Apply a single event to projection state.

  ## Parameters
    * `projection` - ProjectionDefinition
    * `state` - Current state
    * `event` - Event to apply

  ## Returns
  New state after applying event

  ## Examples

      iex> projection = Definition.new(
      ...>   name: :counter,
      ...>   version: 1,
      ...>   initial_state: 0,
      ...>   project_fn: fn state, _event -> state + 1 end
      ...> )
      iex> Projection.apply_event(projection, 0, %{})
      1
  """
  def apply_event(%Definition{project_fn: project_fn}, state, event) do
    project_fn.(state, event)
  end

  @doc """
  Apply multiple events to projection state.

  ## Parameters
    * `projection` - ProjectionDefinition
    * `state` - Initial state
    * `events` - List of events to apply

  ## Returns
  Final state after applying all events

  ## Examples

      iex> projection = Definition.new(
      ...>   name: :counter,
      ...>   version: 1,
      ...>   initial_state: 0,
      ...>   project_fn: fn state, _event -> state + 1 end
      ...> )
      iex> Projection.apply_events(projection, 0, [%{}, %{}, %{}])
      3
  """
  def apply_events(projection, state, events) when is_list(events) do
    Enum.reduce(events, state, fn event, acc ->
      apply_event(projection, acc, event)
    end)
  end

  @doc "Get initial state for a projection"
  def initialize_state(%Definition{initial_state: initial_state}) do
    initial_state
  end

  ## State Migration

  @doc """
  Migrate projection state using a migration function.

  ## Parameters
    * `old_state` - State from previous version
    * `migration_fn` - Function to transform old state to new state

  ## Returns
  Migrated state
  """
  def migrate_state(old_state, migration_fn) when is_function(migration_fn, 1) do
    migration_fn.(old_state)
  end

  ## Accessors

  @doc "Get projection name"
  def get_name(%Definition{name: name}), do: name

  @doc "Get projection version"
  def get_version(%Definition{version: version}), do: version

  @doc "Get projection initial state"
  def get_initial_state(%Definition{initial_state: initial_state}), do: initial_state

  @doc "Get projection metadata"
  def get_metadata(%Definition{metadata: metadata}), do: metadata

  ## State Snapshot Management

  @doc """
  Create a snapshot of current projection state.

  ## Parameters
    * `projection_name` - Name of the projection
    * `entity_id` - Entity ID
    * `state` - Current state
    * `version` - Projection version

  ## Returns
  ProjectionSnapshot
  """
  def snapshot_state(projection_name, entity_id, state, version) do
    Snapshot.new(
      projection_name: projection_name,
      entity_id: entity_id,
      state: state,
      version: version,
      timestamp: DateTime.utc_now()
    )
  end

  @doc "Restore state from a snapshot"
  def restore_from_snapshot(%Snapshot{state: state}), do: state
end
