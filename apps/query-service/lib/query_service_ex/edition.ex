defmodule QueryServiceEx.Edition do
  @moduledoc """
  Edition detection for community vs enterprise features.

  Set via the `ALLSOURCE_EDITION` environment variable (default: "community").
  Enterprise mode enables tenant management, quota enforcement, and billing.
  """

  def current do
    Application.get_env(:query_service_ex, :edition, :community)
  end

  def enterprise? do
    current() == :enterprise
  end

  def community? do
    current() == :community
  end
end
