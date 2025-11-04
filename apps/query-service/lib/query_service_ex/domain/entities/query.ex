defmodule QueryServiceEx.Domain.Entities.Query do
  @moduledoc """
  Domain entity for query representation.
  Pure data structures with no external dependencies.

  Represents a query against the event store with support for:
  - Field selection
  - WHERE predicates
  - ORDER BY clauses
  - LIMIT and OFFSET
  - Aggregations
  """

  @type operator ::
          :eq | :ne | :gt | :gte | :lt | :lte |
          :contains | :in | :not_in | :between |
          :and | :or | :not

  @type aggregation_function ::
          :count | :sum | :avg | :min | :max |
          :count_distinct | :percentile | :first | :last

  @type sort_direction :: :asc | :desc

  defmodule Predicate do
    @moduledoc "Represents a condition in a WHERE clause"
    @type t :: %__MODULE__{
            operator: QueryServiceEx.Domain.Entities.Query.operator(),
            field: atom() | nil,
            value: any()
          }

    @enforce_keys [:operator]
    defstruct [:operator, :field, :value]

    @doc "Create a new predicate"
    def new(operator, field \\ nil, value \\ nil) do
      %__MODULE__{
        operator: operator,
        field: field,
        value: value
      }
    end
  end

  defmodule Aggregation do
    @moduledoc "Represents an aggregation function"
    @type t :: %__MODULE__{
            function: QueryServiceEx.Domain.Entities.Query.aggregation_function(),
            field: atom(),
            alias: atom()
          }

    @enforce_keys [:function, :field, :alias]
    defstruct [:function, :field, :alias]

    @doc "Create a new aggregation"
    def new(function, field, field_alias) do
      %__MODULE__{
        function: function,
        field: field,
        alias: field_alias
      }
    end
  end

  defmodule SortOrder do
    @moduledoc "Represents a sort order specification"
    @type t :: %__MODULE__{
            field: atom(),
            direction: QueryServiceEx.Domain.Entities.Query.sort_direction()
          }

    @enforce_keys [:field, :direction]
    defstruct [:field, :direction]

    @doc "Create a new sort order"
    def new(field, direction \\ :asc) do
      %__MODULE__{
        field: field,
        direction: direction
      }
    end
  end

  @type t :: %__MODULE__{
          select: list(atom()),
          from: atom(),
          where: Predicate.t() | nil,
          order_by: list(SortOrder.t()) | nil,
          limit: pos_integer() | nil,
          offset: non_neg_integer() | nil,
          aggregations: list(Aggregation.t()) | nil
        }

  defstruct select: [:*],
            from: :events,
            where: nil,
            order_by: nil,
            limit: nil,
            offset: nil,
            aggregations: nil

  # Domain constants
  @operators ~w(eq ne gt gte lt lte contains in not_in between and or not)a
  @aggregation_functions ~w(count sum avg min max count_distinct percentile first last)a
  @sort_directions ~w(asc desc)a

  ## Validation Functions

  @doc "Check if operator is valid"
  def valid_operator?(op) when op in @operators, do: true
  def valid_operator?(_), do: false

  @doc "Check if aggregation function is valid"
  def valid_aggregation_function?(fn_name) when fn_name in @aggregation_functions, do: true
  def valid_aggregation_function?(_), do: false

  @doc "Check if sort direction is valid"
  def valid_sort_direction?(dir) when dir in @sort_directions, do: true
  def valid_sort_direction?(_), do: false

  @doc "Validate a predicate"
  def valid_predicate?(%Predicate{operator: op, field: field}) do
    valid_operator?(op) && (is_nil(field) || is_atom(field))
  end

  def valid_predicate?(_), do: false

  @doc "Validate a query"
  def valid_query?(%__MODULE__{from: from, where: where}) do
    (is_nil(from) || is_atom(from)) &&
      (is_nil(where) || valid_predicate?(where))
  end

  def valid_query?(_), do: false

  ## Constructor Functions

  @doc """
  Create a new query entity.

  ## Options
    * `:select` - List of fields to select (default: `[:*]`)
    * `:from` - Table/source to query (default: `:events`)
    * `:where` - Predicate for filtering
    * `:order_by` - List of sort orders
    * `:limit` - Maximum number of results
    * `:offset` - Number of results to skip
    * `:aggregations` - List of aggregations

  ## Examples

      iex> query = Query.new(from: :events, limit: 100)
      iex> query.from
      :events
      iex> query.limit
      100
  """
  def new(opts \\ []) do
    struct(__MODULE__, opts)
  end

  @doc "Create a new predicate"
  defdelegate make_predicate(operator, field, value), to: Predicate, as: :new

  @doc "Create a new aggregation"
  defdelegate make_aggregation(function, field, field_alias), to: Aggregation, as: :new

  @doc "Create a new sort order"
  defdelegate make_sort_order(field, direction), to: SortOrder, as: :new

  ## Query Composition

  @doc "Add a WHERE predicate to a query"
  def add_predicate(%__MODULE__{} = query, %Predicate{} = predicate) do
    if valid_predicate?(predicate) do
      %{query | where: predicate}
    else
      raise ArgumentError, "Invalid predicate: #{inspect(predicate)}"
    end
  end

  @doc "Add ORDER BY clause to a query"
  def add_order_by(%__MODULE__{} = query, sort_orders) when is_list(sort_orders) do
    %{query | order_by: sort_orders}
  end

  @doc "Add LIMIT clause to a query"
  def add_limit(%__MODULE__{} = query, n) when is_integer(n) and n > 0 do
    %{query | limit: n}
  end

  @doc "Add OFFSET clause to a query"
  def add_offset(%__MODULE__{} = query, n) when is_integer(n) and n >= 0 do
    %{query | offset: n}
  end

  @doc "Combine multiple predicates with AND or OR"
  def combine_predicates(operator, predicates) when operator in [:and, :or] and is_list(predicates) do
    Predicate.new(operator, nil, predicates)
  end

  @doc "Set SELECT fields"
  def select(%__MODULE__{} = query, fields) when is_list(fields) do
    %{query | select: fields}
  end
end
