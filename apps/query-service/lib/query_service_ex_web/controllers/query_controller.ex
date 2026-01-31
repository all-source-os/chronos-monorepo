defmodule QueryServiceExWeb.QueryController do
  @moduledoc """
  Controller for executing queries using the Query DSL.
  Uses FallbackController for consistent error handling.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Domain.Entities.Query
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  action_fallback(QueryServiceExWeb.FallbackController)

  @doc """
  Execute a query.

  Body supports two formats:

  1. Simple filter parameters:
  {
    "entity_id": "string",
    "event_type": "string",
    "limit": 100,
    "offset": 0
  }

  2. DSL query (advanced):
  {
    "from": "events",
    "where": {
      "op": "eq",
      "field": "event_type",
      "value": "order.placed"
    },
    "limit": 100
  }
  """
  def execute(conn, params) do
    # Check if this is a simple query or DSL query
    query =
      if Map.has_key?(params, "from") or Map.has_key?(params, "where") do
        build_dsl_query(params)
      else
        build_simple_query(params)
      end

    case query do
      {:ok, q} ->
        case RustCoreClient.query_events(q) do
          {:ok, events} ->
            json(conn, %{
              data: events,
              count: length(events),
              query: summarize_query(q)
            })

          {:error, reason} ->
            conn
            |> put_status(:bad_request)
            |> json(%{error: to_string(reason)})
        end

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: reason})
    end
  end

  # Build a DSL query from parameters
  defp build_dsl_query(params) do
    try do
      query = Query.new(from: String.to_atom(params["from"] || "events"))

      query =
        if where = params["where"] do
          predicate = build_predicate(where)
          Query.add_predicate(query, predicate)
        else
          query
        end

      query =
        if limit = params["limit"] do
          Query.add_limit(query, limit)
        else
          query
        end

      query =
        if offset = params["offset"] do
          Query.add_offset(query, offset)
        else
          query
        end

      {:ok, query}
    rescue
      e -> {:error, Exception.message(e)}
    end
  end

  # Build a simple query from basic parameters
  defp build_simple_query(params) do
    {:ok, params}
  end

  # Build a predicate from JSON
  defp build_predicate(%{"op" => op, "field" => field, "value" => value}) do
    op_atom = String.to_atom(op)
    field_atom = String.to_atom(field)
    Query.make_predicate(op_atom, field_atom, value)
  end

  defp build_predicate(%{"and" => predicates}) when is_list(predicates) do
    preds = Enum.map(predicates, &build_predicate/1)
    Query.combine_predicates(:and, preds)
  end

  defp build_predicate(%{"or" => predicates}) when is_list(predicates) do
    preds = Enum.map(predicates, &build_predicate/1)
    Query.combine_predicates(:or, preds)
  end

  # Summarize query for response
  defp summarize_query(%Query{} = query) do
    %{
      from: query.from,
      has_filter: query.where != nil,
      limit: query.limit,
      offset: query.offset
    }
  end

  defp summarize_query(params) when is_map(params) do
    %{
      type: "simple",
      filters: Map.keys(params)
    }
  end
end
