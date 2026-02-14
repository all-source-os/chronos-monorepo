defmodule QueryServiceExWeb.ConnCase do
  @moduledoc """
  This module defines the test case to be used by
  tests that require setting up a connection.
  """

  use ExUnit.CaseTemplate

  using do
    quote do
      import Plug.Conn
      import Plug.Test
      import Phoenix.ConnTest
      import QueryServiceExWeb.ConnCase

      alias QueryServiceExWeb.Router.Helpers, as: Routes

      @endpoint QueryServiceExWeb.Endpoint
    end
  end

  setup _tags do
    {:ok, conn: Phoenix.ConnTest.build_conn()}
  end
end
