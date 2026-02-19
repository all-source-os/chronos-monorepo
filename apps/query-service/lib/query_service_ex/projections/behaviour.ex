defmodule QueryServiceEx.Projections.Behaviour do
  @moduledoc """
  Defines the contract for server-side projection modules.

  Each projection module maps a stream of events for a given entity type
  into projected state via a left fold: `initial_state |> apply_event(e1) |> apply_event(e2) ...`

  Projection modules are registered at compile time in `QueryServiceEx.Projections.Registry`.
  """

  @doc ~s(The entity type this projection folds over, e.g. "index" or "trade".)
  @callback entity_type() :: String.t()

  @doc "The initial accumulator state before any events are applied."
  @callback initial_state() :: map()

  @doc "Apply a single event to the current state, returning the new state."
  @callback apply_event(state :: map(), event :: map()) :: map()

  @doc "Fields on the projected state that clients may filter on."
  @callback filterable_fields() :: [String.t()]
end
