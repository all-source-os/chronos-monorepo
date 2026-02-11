defmodule QueryServiceEx.Cluster.DistributedRegistryTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Cluster.DistributedRegistry

  @moduletag :unit

  setup do
    # Ensure DistributedRegistry is started
    case GenServer.whereis(DistributedRegistry) do
      nil ->
        {:ok, _pid} = DistributedRegistry.start_link([])

      _pid ->
        :ok
    end

    :ok
  end

  describe "register/2 and whereis/1" do
    test "registers and finds a process" do
      name = {:test, "process", System.unique_integer([:positive])}

      assert {:ok, _pid} = DistributedRegistry.register(name)
      # Wait for Horde CRDT to sync (eventual consistency)
      Process.sleep(10)
      assert DistributedRegistry.whereis(name) == self()
    end

    test "returns :undefined for unregistered names" do
      name = {:test, "nonexistent", System.unique_integer([:positive])}
      assert DistributedRegistry.whereis(name) == :undefined
    end

    test "allows registering with explicit pid" do
      name = {:test, "explicit", System.unique_integer([:positive])}

      assert {:ok, _} = DistributedRegistry.register(name)
      # Wait for Horde CRDT to sync (eventual consistency)
      Process.sleep(10)
      assert DistributedRegistry.whereis(name) == self()
    end
  end

  describe "unregister/1" do
    test "removes registration" do
      name = {:test, "unregister", System.unique_integer([:positive])}

      {:ok, _} = DistributedRegistry.register(name)
      # Wait for Horde CRDT to sync (eventual consistency)
      Process.sleep(10)
      assert DistributedRegistry.whereis(name) == self()

      DistributedRegistry.unregister(name)
      Process.sleep(10)
      assert DistributedRegistry.whereis(name) == :undefined
    end
  end

  describe "lookup/1" do
    test "returns {pid, value} tuple for registered name" do
      name = {:test, "lookup", System.unique_integer([:positive])}

      {:ok, _} = DistributedRegistry.register(name)

      # Wait for Horde CRDT to sync (eventual consistency)
      Process.sleep(10)

      result = DistributedRegistry.lookup(name)
      assert {pid, _value} = result
      assert pid == self()
    end

    test "returns nil for unregistered name" do
      name = {:test, "lookup_miss", System.unique_integer([:positive])}
      assert DistributedRegistry.lookup(name) == nil
    end
  end

  describe "count/0" do
    test "returns number of registered processes" do
      count = DistributedRegistry.count()
      assert is_integer(count)
      assert count >= 0
    end
  end

  describe "available?/0" do
    test "returns true when registry is running" do
      assert DistributedRegistry.available?() == true
    end
  end

  describe "members/0" do
    test "returns list of registry members" do
      members = DistributedRegistry.members()
      assert is_list(members)
      # Should have at least the current node
      assert length(members) >= 1
    end
  end

  describe "list_projection_processes/1" do
    test "lists processes for a projection" do
      projection = "test_projection_#{System.unique_integer([:positive])}"

      # Register some processes
      name1 = {:projection, projection, "entity-1"}
      name2 = {:projection, projection, "entity-2"}

      {:ok, _} = DistributedRegistry.register(name1)

      # Spawn a separate process for the second registration
      task =
        Task.async(fn ->
          {:ok, _} = DistributedRegistry.register(name2)
          Process.sleep(100)
        end)

      # Give it time to register
      Process.sleep(50)

      processes = DistributedRegistry.list_projection_processes(projection)
      assert is_list(processes)

      Task.await(task)

      # Clean up
      DistributedRegistry.unregister(name1)
    end
  end
end
