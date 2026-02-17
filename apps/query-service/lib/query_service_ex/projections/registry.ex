defmodule QueryServiceEx.Projections.Registry do
  @moduledoc """
  Compile-time registry mapping projection names to their implementing modules.

  No dynamic code loading — every projection is a known module at compile time.
  """

  @projections %{
    "indices" => QueryServiceEx.Projections.IndexState,
    "trades" => QueryServiceEx.Projections.TradeState,
    "portfolios" => QueryServiceEx.Projections.PortfolioState,
    "sagas" => QueryServiceEx.Projections.SagaState
  }

  @doc "Returns the projection module for the given name, or nil if not registered."
  @spec get(String.t()) :: module() | nil
  def get(name), do: Map.get(@projections, name)

  @doc "Returns the list of registered projection names."
  @spec list() :: [String.t()]
  def list, do: Map.keys(@projections)
end
