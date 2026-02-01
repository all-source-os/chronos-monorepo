defmodule QueryServiceEx.Domain.Entities.Pipeline do
  @moduledoc """
  Domain entity for event processing pipelines.

  A pipeline transforms event streams through composable operators:
  - Filter: Remove events based on predicates
  - Transform: Modify event structure/data
  - Enrich: Add external data to events
  - Aggregate: Combine events in time windows
  - Route: Send events to different destinations
  """

  defmodule Definition do
    @moduledoc "Pipeline definition entity"

    @type operator_type :: :filter | :transform | :enrich | :aggregate | :route | :validate
    @type operator :: %{
            type: operator_type(),
            name: String.t(),
            config: map(),
            function: (any() -> any()) | nil
          }

    @type t :: %__MODULE__{
            name: atom(),
            version: pos_integer(),
            operators: list(operator()),
            config: map(),
            metadata: map()
          }

    @enforce_keys [:name, :version, :operators]
    defstruct [:name, :version, :operators, config: %{}, metadata: %{}]

    @doc "Create a new pipeline definition"
    def new(opts) do
      struct!(__MODULE__, opts)
    end
  end

  ## Validation Functions

  @doc "Check if version is valid (positive integer)"
  def valid_version?(version) when is_integer(version) and version > 0, do: true
  def valid_version?(_), do: false

  @doc "Check if pipeline name is valid (must be atom)"
  def valid_name?(name) when is_atom(name), do: true
  def valid_name?(_), do: false

  @doc "Check if operator type is valid"
  def valid_operator_type?(type)
      when type in [:filter, :transform, :enrich, :aggregate, :route, :validate],
      do: true

  def valid_operator_type?(_), do: false

  @doc "Validate an operator"
  def valid_operator?(%{type: type, name: name}) when is_binary(name) do
    valid_operator_type?(type)
  end

  def valid_operator?(_), do: false

  @doc "Validate a complete pipeline definition"
  def valid_pipeline?(%Definition{name: name, version: version, operators: operators}) do
    valid_name?(name) and valid_version?(version) and is_list(operators) and
      Enum.all?(operators, &valid_operator?/1)
  end

  def valid_pipeline?(_), do: false

  ## Pipeline Operators

  @doc """
  Create a filter operator.

  Filters events based on a predicate function.

  ## Examples

      iex> filter_op = Pipeline.filter_operator("only_orders", fn event ->
      ...>   event.event_type == "order.placed"
      ...> end)
  """
  def filter_operator(name, predicate_fn) when is_function(predicate_fn, 1) do
    %{
      type: :filter,
      name: name,
      config: %{},
      function: predicate_fn
    }
  end

  @doc """
  Create a transform operator.

  Transforms events by applying a function.

  ## Examples

      iex> transform_op = Pipeline.transform_operator("add_timestamp", fn event ->
      ...>   Map.put(event, :processed_at, DateTime.utc_now())
      ...> end)
  """
  def transform_operator(name, transform_fn) when is_function(transform_fn, 1) do
    %{
      type: :transform,
      name: name,
      config: %{},
      function: transform_fn
    }
  end

  @doc """
  Create an enrich operator.

  Enriches events with external data.

  ## Examples

      iex> enrich_op = Pipeline.enrich_operator("add_user_data", fn event ->
      ...>   Map.put(event, :user, %{id: event.user_id, name: "John"})
      ...> end)
      iex> enrich_op.type
      :enrich
  """
  def enrich_operator(name, enrich_fn) when is_function(enrich_fn, 1) do
    %{
      type: :enrich,
      name: name,
      config: %{},
      function: enrich_fn
    }
  end

  @doc """
  Create a validate operator.

  Validates events against rules.
  """
  def validate_operator(name, validate_fn) when is_function(validate_fn, 1) do
    %{
      type: :validate,
      name: name,
      config: %{},
      function: validate_fn
    }
  end

  @doc """
  Create a route operator.

  Routes events to different destinations.
  """
  def route_operator(name, route_fn) when is_function(route_fn, 1) do
    %{
      type: :route,
      name: name,
      config: %{},
      function: route_fn
    }
  end

  ## Pipeline Execution

  @doc """
  Apply a single operator to an event.

  Returns `{:ok, event}` if successful, `{:error, reason}` if failed,
  or `:filtered` if the event should be filtered out.
  """
  def apply_operator(event, %{type: :filter, function: fn_}) do
    if fn_.(event) do
      {:ok, event}
    else
      :filtered
    end
  end

  def apply_operator(event, %{type: type, function: fn_})
      when type in [:transform, :enrich, :validate] do
    {:ok, fn_.(event)}
  rescue
    e -> {:error, Exception.message(e)}
  end

  def apply_operator(event, %{type: :route, function: fn_}) do
    destination = fn_.(event)
    {:ok, Map.put(event, :__route__, destination)}
  rescue
    e -> {:error, Exception.message(e)}
  end

  @doc """
  Apply a pipeline to an event.

  Returns `{:ok, event}` if successful, `{:error, reason}` if failed,
  or `:filtered` if the event was filtered out.
  """
  def apply_pipeline(event, %Definition{operators: operators}) do
    Enum.reduce_while(operators, {:ok, event}, fn operator, {:ok, current_event} ->
      case apply_operator(current_event, operator) do
        {:ok, new_event} -> {:cont, {:ok, new_event}}
        :filtered -> {:halt, :filtered}
        {:error, _} = error -> {:halt, error}
      end
    end)
  end

  @doc """
  Apply a pipeline to multiple events.

  Returns a list of successfully processed events.
  """
  def apply_pipeline_batch(events, pipeline) when is_list(events) do
    events
    |> Enum.map(&apply_pipeline(&1, pipeline))
    |> Enum.filter(fn
      {:ok, _} -> true
      _ -> false
    end)
    |> Enum.map(fn {:ok, event} -> event end)
  end

  ## Accessors

  @doc "Get pipeline name"
  def get_name(%Definition{name: name}), do: name

  @doc "Get pipeline version"
  def get_version(%Definition{version: version}), do: version

  @doc "Get pipeline operators"
  def get_operators(%Definition{operators: operators}), do: operators

  @doc "Get pipeline config"
  def get_config(%Definition{config: config}), do: config

  @doc "Get pipeline metadata"
  def get_metadata(%Definition{metadata: metadata}), do: metadata

  @doc "Count operators in pipeline"
  def operator_count(%Definition{operators: operators}), do: length(operators)
end
