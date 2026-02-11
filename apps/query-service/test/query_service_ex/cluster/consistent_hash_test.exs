defmodule QueryServiceEx.Cluster.ConsistentHashTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Cluster.ConsistentHash

  @moduletag :unit

  setup do
    # Ensure ConsistentHash is started
    case GenServer.whereis(ConsistentHash) do
      nil ->
        {:ok, _pid} = ConsistentHash.start_link([])

      _pid ->
        :ok
    end

    :ok
  end

  describe "get_node/1" do
    test "returns a node for a key" do
      node = ConsistentHash.get_node("entity-123")
      assert is_atom(node)
    end

    test "returns same node for same key (consistency)" do
      key = "entity-#{System.unique_integer([:positive])}"

      node1 = ConsistentHash.get_node(key)
      node2 = ConsistentHash.get_node(key)
      node3 = ConsistentHash.get_node(key)

      assert node1 == node2
      assert node2 == node3
    end

    test "handles binary keys" do
      node = ConsistentHash.get_node("my-entity-id")
      assert is_atom(node)
    end

    test "handles non-binary keys by converting to string" do
      node = ConsistentHash.get_node(12_345)
      assert is_atom(node)
    end
  end

  describe "get_node_for_projection/2" do
    test "returns a node for projection/entity combination" do
      node = ConsistentHash.get_node_for_projection("user_balance", "user-123")
      assert is_atom(node)
    end

    test "same projection/entity always routes to same node" do
      projection = "my_projection"
      entity_id = "entity-#{System.unique_integer([:positive])}"

      node1 = ConsistentHash.get_node_for_projection(projection, entity_id)
      node2 = ConsistentHash.get_node_for_projection(projection, entity_id)

      assert node1 == node2
    end

    test "different entities may route to different nodes" do
      projection = "my_projection"

      # Generate many entity IDs to increase chance of different routing
      nodes =
        for i <- 1..100 do
          ConsistentHash.get_node_for_projection(projection, "entity-#{i}")
        end

      # In single-node mode, all go to same node
      # Just verify they're all valid nodes
      assert Enum.all?(nodes, &is_atom/1)
    end
  end

  describe "local?/1" do
    test "returns boolean" do
      result = ConsistentHash.local?("entity-123")
      assert is_boolean(result)
    end

    test "in single-node mode, everything is local" do
      # When there's only one node, all keys should be local
      if Enum.empty?(Node.list()) do
        assert ConsistentHash.local?("any-entity")
      end
    end
  end

  describe "local_for_projection?/2" do
    test "returns boolean" do
      result = ConsistentHash.local_for_projection?("projection", "entity")
      assert is_boolean(result)
    end
  end

  describe "nodes/0" do
    test "returns list of nodes including self" do
      nodes = ConsistentHash.nodes()
      assert is_list(nodes)
      assert node() in nodes
    end
  end

  describe "node_count/0" do
    test "returns at least 1" do
      count = ConsistentHash.node_count()
      assert count >= 1
    end

    test "equals length of nodes list" do
      count = ConsistentHash.node_count()
      nodes = ConsistentHash.nodes()
      assert count == length(nodes)
    end
  end

  describe "rebuild/0" do
    test "rebuilds the hash ring" do
      assert :ok = ConsistentHash.rebuild()
    end

    test "maintains consistency after rebuild" do
      key = "test-entity-#{System.unique_integer([:positive])}"

      node_before = ConsistentHash.get_node(key)
      ConsistentHash.rebuild()
      node_after = ConsistentHash.get_node(key)

      # With same nodes, key should route to same place
      assert node_before == node_after
    end
  end

  describe "keys_for_node/1" do
    test "returns ownership info for existing node" do
      info = ConsistentHash.keys_for_node(node())

      assert Map.has_key?(info, :node)
      assert Map.has_key?(info, :total_nodes)
      assert Map.has_key?(info, :approximate_share)
      assert info.total_nodes >= 1
    end

    test "returns zero share for non-existent node" do
      info = ConsistentHash.keys_for_node(:nonexistent@host)

      assert info.approximate_share == 0.0
    end
  end
end
