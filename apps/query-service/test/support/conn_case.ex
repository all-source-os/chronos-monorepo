defmodule QueryServiceExWeb.ConnCase do
  @moduledoc """
  This module defines the test case to be used by
  tests that require setting up a connection.

  Such tests rely on `Phoenix.ConnTest` and also
  import other functionality to make it easier
  to build common data structures and query the data layer.
  """

  use ExUnit.CaseTemplate

  using do
    quote do
      # Import conveniences for testing with connections
      import Plug.Conn
      import Plug.Test
      import Phoenix.ConnTest
      import QueryServiceExWeb.ConnCase

      alias QueryServiceExWeb.Router.Helpers, as: Routes

      # The default endpoint for testing
      @endpoint QueryServiceExWeb.Endpoint
    end
  end

  setup tags do
    :ok = Ecto.Adapters.SQL.Sandbox.checkout(QueryServiceEx.Repo)

    unless tags[:async] do
      Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, {:shared, self()})
    end

    {:ok, conn: Phoenix.ConnTest.build_conn()}
  end
end
