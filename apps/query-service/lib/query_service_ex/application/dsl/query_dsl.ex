defmodule QueryServiceEx.Application.DSL.QueryDSL do
  @moduledoc """
  Query DSL for building queries in a fluent, declarative style.

  This is the main user-facing API that provides a natural Elixir syntax
  for building event store queries.

  ## Examples

      # Simple query
      query
      |> from_events()
      |> where(event_type == "user.created")
      |> limit(100)

      # Pipe-friendly fluent API
      from_events()
      |> select([:entity_id, :event_type, :timestamp])
      |> where(event_type == "order.placed" and timestamp > days_ago(7))
      |> order_by([{:timestamp, :desc}])
      |> limit(50)

      # Time-based query
      from_events()
      |> where(event_type == "order.placed")
      |> since(days_ago(30))
      |> order_by_timestamp(:desc)
      |> take(1000)
  """

  alias QueryServiceEx.Domain.Entities.Query
  alias QueryServiceEx.Domain.Entities.Query.{Aggregation, Predicate, SortOrder}

  ## Query Builders

  @doc "Start building a query from events table"
  def from_events do
    Query.new(from: :events)
  end

  @doc """
  Start building a query from a projection.

  ## Examples

      iex> from_projection(:user_statistics)
      %Query{from: :user_statistics, ...}
  """
  def from_projection(projection_name) when is_atom(projection_name) do
    Query.new(from: projection_name)
  end

  ## WHERE Clause Builders

  @doc """
  Add a WHERE clause to a query using pattern matching.

  ## Examples

      iex> query |> where(event_type: "user.created")
      iex> query |> where(eq(:event_type, "user.created"))
  """
  def where(query, predicate) when is_list(predicate) do
    # Handle keyword list
    predicates =
      Enum.map(predicate, fn {field, value} ->
        Predicate.new(:eq, field, value)
      end)

    combined =
      if length(predicates) == 1 do
        hd(predicates)
      else
        Query.combine_predicates(:and, predicates)
      end

    Query.add_predicate(query, combined)
  end

  def where(query, {op, field, value}) do
    predicate = Predicate.new(op, field, value)
    Query.add_predicate(query, predicate)
  end

  ## Comparison Operators (Helper Functions)

  @doc "Equal comparison helper"
  def eq(field, value), do: {:eq, field, value}

  @doc "Not equal comparison helper"
  def ne(field, value), do: {:ne, field, value}

  @doc "Greater than comparison helper"
  def gt(field, value), do: {:gt, field, value}

  @doc "Greater than or equal comparison helper"
  def gte(field, value), do: {:gte, field, value}

  @doc "Less than comparison helper"
  def lt(field, value), do: {:lt, field, value}

  @doc "Less than or equal comparison helper"
  def lte(field, value), do: {:lte, field, value}

  @doc "String contains comparison helper"
  def contains?(field, substring), do: {:contains, field, substring}

  @doc "IN operator helper (value in list)"
  def in?(field, values), do: {:in, field, values}

  @doc "BETWEEN operator helper"
  def between(field, lower, upper), do: {:between, field, {lower, upper}}

  ## Logical Operators

  @doc "Combine predicates with AND"
  def and_predicates(predicates) when is_list(predicates) do
    Query.combine_predicates(:and, predicates)
  end

  @doc "Combine predicates with OR"
  def or_predicates(predicates) when is_list(predicates) do
    Query.combine_predicates(:or, predicates)
  end

  ## SELECT Clause

  @doc """
  Specify which fields to select.

  ## Examples

      iex> query |> select([:entity_id, :event_type, :timestamp])
  """
  def select(query, fields) when is_list(fields) do
    Query.select(query, fields)
  end

  @doc "Select all fields"
  def select_all(query) do
    Query.select(query, [:*])
  end

  ## ORDER BY Clause

  @doc """
  Add ORDER BY clause.

  ## Examples

      iex> query |> order_by([{:timestamp, :desc}])
      iex> query |> order_by([{:timestamp, :desc}, {:entity_id, :asc}])
  """
  def order_by(query, order_spec) when is_list(order_spec) do
    sort_orders =
      Enum.map(order_spec, fn {field, dir} ->
        SortOrder.new(field, dir || :asc)
      end)

    Query.add_order_by(query, sort_orders)
  end

  @doc """
  Order by timestamp.

  ## Examples

      iex> query |> order_by_timestamp()  # defaults to :desc
      iex> query |> order_by_timestamp(:asc)
  """
  def order_by_timestamp(query, direction \\ :desc) do
    order_by(query, [{:timestamp, direction}])
  end

  ## LIMIT and OFFSET

  @doc """
  Add LIMIT clause.

  ## Examples

      iex> query |> limit(100)
  """
  def limit(query, n) when is_integer(n) and n > 0 do
    Query.add_limit(query, n)
  end

  @doc """
  Add OFFSET clause for pagination.

  ## Examples

      iex> query |> limit(100) |> offset(200)
  """
  def offset(query, n) when is_integer(n) and n >= 0 do
    Query.add_offset(query, n)
  end

  @doc "Alias for limit"
  def take(query, n), do: limit(query, n)

  ## Time-based Helpers

  @doc "Current timestamp"
  def now, do: DateTime.utc_now()

  @doc "Timestamp n days ago"
  def days_ago(n) when is_integer(n) do
    DateTime.utc_now()
    |> DateTime.add(-n * 24 * 60 * 60, :second)
  end

  @doc "Timestamp n hours ago"
  def hours_ago(n) when is_integer(n) do
    DateTime.utc_now()
    |> DateTime.add(-n * 60 * 60, :second)
  end

  @doc "Timestamp n minutes ago"
  def minutes_ago(n) when is_integer(n) do
    DateTime.utc_now()
    |> DateTime.add(-n * 60, :second)
  end

  @doc """
  Events since timestamp.

  ## Examples

      iex> query |> since(days_ago(7))
  """
  def since(query, timestamp) do
    where(query, gt(:timestamp, timestamp))
  end

  @doc "Events until timestamp"
  def until(query, timestamp) do
    where(query, lt(:timestamp, timestamp))
  end

  @doc "Events in time range"
  def time_range(query, start_time, end_time) do
    query
    |> where(gte(:timestamp, start_time))
    |> where(lte(:timestamp, end_time))
  end

  ## Aggregation Helpers

  @doc "Count aggregation"
  def count_events(field \\ :*) do
    Aggregation.new(:count, field, :count)
  end

  @doc "Sum aggregation"
  def sum_field(field) do
    alias_name = String.to_atom("#{field}_sum")
    Aggregation.new(:sum, field, alias_name)
  end

  @doc "Average aggregation"
  def avg_field(field) do
    alias_name = String.to_atom("#{field}_avg")
    Aggregation.new(:avg, field, alias_name)
  end

  @doc "Minimum aggregation"
  def min_field(field) do
    alias_name = String.to_atom("#{field}_min")
    Aggregation.new(:min, field, alias_name)
  end

  @doc "Maximum aggregation"
  def max_field(field) do
    alias_name = String.to_atom("#{field}_max")
    Aggregation.new(:max, field, alias_name)
  end

  @doc "Count distinct values"
  def count_distinct(field) do
    Aggregation.new(:count_distinct, field, :distinct_count)
  end

  ## Convenience Macros for Query Building

  defmacro query(do: block) do
    quote do
      from_events()
      |> unquote(block)
    end
  end
end
