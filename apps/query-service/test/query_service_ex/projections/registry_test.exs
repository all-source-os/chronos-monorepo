defmodule QueryServiceEx.Projections.RegistryTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.Registry

  describe "list/0" do
    test "returns a list" do
      assert is_list(Registry.list())
    end
  end

  describe "get/1" do
    test "returns nil for unregistered projection" do
      assert Registry.get("nonexistent") == nil
    end
  end
end
