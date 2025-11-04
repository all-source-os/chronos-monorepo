defmodule QueryServiceExTest do
  use ExUnit.Case
  doctest QueryServiceEx

  test "greets the world" do
    assert QueryServiceEx.hello() == :world
  end
end
