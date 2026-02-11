defmodule QueryServiceEx.Cluster.TopologyTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Cluster.Topology

  @moduletag :unit

  describe "strategy/0" do
    test "returns 'none' by default" do
      # Default when CLUSTER_STRATEGY is not set
      assert Topology.strategy() == System.get_env("CLUSTER_STRATEGY", "none")
    end
  end

  describe "enabled?/0" do
    test "returns false when strategy is 'none'" do
      # This test depends on current env setting
      if Topology.strategy() == "none" do
        refute Topology.enabled?()
      else
        assert Topology.enabled?()
      end
    end
  end

  describe "connected_nodes/0" do
    test "returns list of connected nodes" do
      nodes = Topology.connected_nodes()
      assert is_list(nodes)
      # In test mode, likely empty unless running distributed tests
    end
  end

  describe "node_count/0" do
    test "returns at least 1 (self)" do
      count = Topology.node_count()
      assert count >= 1
    end
  end

  describe "child_spec/0" do
    test "returns nil when strategy is 'none'" do
      # Save original value
      original = System.get_env("CLUSTER_STRATEGY")

      try do
        System.put_env("CLUSTER_STRATEGY", "none")
        assert Topology.child_spec() == nil
      after
        # Restore original value
        if original do
          System.put_env("CLUSTER_STRATEGY", original)
        else
          System.delete_env("CLUSTER_STRATEGY")
        end
      end
    end

    test "returns child_spec for kubernetes strategy" do
      original = System.get_env("CLUSTER_STRATEGY")

      try do
        System.put_env("CLUSTER_STRATEGY", "kubernetes")
        System.put_env("CLUSTER_KUBERNETES_NAMESPACE", "test")
        System.put_env("CLUSTER_KUBERNETES_SELECTOR", "app=test")

        spec = Topology.child_spec()
        assert spec != nil
        assert is_tuple(spec)
        assert elem(spec, 0) == Cluster.Supervisor
      after
        if original do
          System.put_env("CLUSTER_STRATEGY", original)
        else
          System.delete_env("CLUSTER_STRATEGY")
        end

        System.delete_env("CLUSTER_KUBERNETES_NAMESPACE")
        System.delete_env("CLUSTER_KUBERNETES_SELECTOR")
      end
    end

    test "returns child_spec for dns strategy" do
      original = System.get_env("CLUSTER_STRATEGY")

      try do
        System.put_env("CLUSTER_STRATEGY", "dns")
        System.put_env("CLUSTER_DNS_QUERY", "test.local")

        spec = Topology.child_spec()
        assert spec != nil
        assert is_tuple(spec)
        assert elem(spec, 0) == Cluster.Supervisor
      after
        if original do
          System.put_env("CLUSTER_STRATEGY", original)
        else
          System.delete_env("CLUSTER_STRATEGY")
        end

        System.delete_env("CLUSTER_DNS_QUERY")
      end
    end

    test "returns child_spec for gossip strategy" do
      original = System.get_env("CLUSTER_STRATEGY")

      try do
        System.put_env("CLUSTER_STRATEGY", "gossip")

        spec = Topology.child_spec()
        assert spec != nil
        assert is_tuple(spec)
        assert elem(spec, 0) == Cluster.Supervisor
      after
        if original do
          System.put_env("CLUSTER_STRATEGY", original)
        else
          System.delete_env("CLUSTER_STRATEGY")
        end
      end
    end

    test "returns nil for unknown strategy" do
      original = System.get_env("CLUSTER_STRATEGY")

      try do
        System.put_env("CLUSTER_STRATEGY", "unknown_strategy")
        assert Topology.child_spec() == nil
      after
        if original do
          System.put_env("CLUSTER_STRATEGY", original)
        else
          System.delete_env("CLUSTER_STRATEGY")
        end
      end
    end
  end
end
