defmodule QueryServiceEx.Prime.GraphFoldTest do
  @moduledoc """
  Folds a small chronological list of `prime.*` events (as they arrive in the
  MAIN multi-tenant event store) into the full-graph contract, asserting the
  exact node/edge/stats shape the dashboard depends on.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.Prime.GraphFold

  # Event payload shapes mirror what allsource-prime emits (verified against
  # apps/core/src/prime): node entity_id = "node:<type>:<uuid>",
  # edge payload carries node-wire source/target, vector entity_id = "vec:<node-wire>".
  defp ev(type, entity_id, payload, ts) do
    %{
      "event_type" => type,
      "entity_id" => entity_id,
      "payload" => payload,
      "timestamp" => ts
    }
  end

  describe "fold/2" do
    test "folds node.created x2, edge.created, node.deleted, vector.stored into the contract" do
      org = "node:organization:org-1"
      person = "node:person:per-1"
      ghost = "node:person:ghost-1"

      events = [
        ev(
          "prime.node.created",
          org,
          %{
            "id" => "org-1",
            "node_type" => "organization",
            "properties" => %{"name" => "Acme", "tenant_id" => "t-1"}
          },
          "2026-01-01T00:00:00Z"
        ),
        ev(
          "prime.node.created",
          person,
          %{
            "id" => "per-1",
            "node_type" => "person",
            "properties" => %{"name" => "Alice"}
          },
          "2026-01-01T00:00:01Z"
        ),
        # A node that gets deleted — must not appear, and its edge must drop.
        ev(
          "prime.node.created",
          ghost,
          %{"id" => "ghost-1", "node_type" => "person", "properties" => %{}},
          "2026-01-01T00:00:02Z"
        ),
        ev(
          "prime.edge.created",
          "edge:edge-1",
          %{
            "id" => "edge-1",
            "source" => person,
            "target" => org,
            "relation" => "works_at",
            "weight" => 0.9,
            "properties" => %{"since" => "2024"}
          },
          "2026-01-01T00:00:03Z"
        ),
        # Edge to the soon-to-be-deleted ghost node — should be dropped.
        ev(
          "prime.edge.created",
          "edge:edge-2",
          %{"id" => "edge-2", "source" => ghost, "target" => org, "relation" => "knows"},
          "2026-01-01T00:00:04Z"
        ),
        ev("prime.node.deleted", ghost, %{"id" => "ghost-1"}, "2026-01-01T00:00:05Z"),
        # Vector for the org node — entity_id is vec:<node-wire>.
        ev(
          "prime.vector.stored",
          "vec:" <> org,
          %{"text" => "Acme corp", "dimensions" => 384, "metadata" => nil},
          "2026-01-01T00:00:06Z"
        )
      ]

      graph = GraphFold.fold(events)

      # --- nodes ---
      assert length(graph["nodes"]) == 2
      ids = Enum.map(graph["nodes"], & &1["id"]) |> Enum.sort()
      assert ids == [org, person]

      org_node = Enum.find(graph["nodes"], &(&1["id"] == org))
      assert org_node["node_type"] == "organization"
      assert org_node["properties"] == %{"name" => "Acme", "tenant_id" => "t-1"}
      assert org_node["has_vector"] == true
      assert org_node["vector_dim"] == 384
      assert org_node["created_at"] == "2026-01-01T00:00:00Z"

      person_node = Enum.find(graph["nodes"], &(&1["id"] == person))
      assert person_node["has_vector"] == false
      assert person_node["vector_dim"] == nil

      # --- edges (edge to deleted ghost dropped) ---
      assert length(graph["edges"]) == 1
      [edge] = graph["edges"]
      assert edge["source"] == person
      assert edge["target"] == org
      assert edge["relation"] == "works_at"
      assert edge["weight"] == 0.9
      assert edge["properties"] == %{"since" => "2024"}

      # --- stats ---
      assert graph["stats"]["node_count"] == 2
      assert graph["stats"]["edge_count"] == 1
      assert graph["stats"]["vector_count"] == 1
      assert graph["stats"]["nodes_by_type"] == %{"organization" => 1, "person" => 1}
      assert graph["has_more"] == false
    end

    test "node.updated merges latest properties, keeps original created_at" do
      n = "node:contact:c-1"

      graph =
        GraphFold.fold([
          ev(
            "prime.node.created",
            n,
            %{"id" => "c-1", "node_type" => "contact", "properties" => %{"status" => "lead"}},
            "2026-01-01T00:00:00Z"
          ),
          ev(
            "prime.node.updated",
            n,
            %{"id" => "c-1", "node_type" => "contact", "properties" => %{"status" => "customer"}},
            "2026-01-02T00:00:00Z"
          )
        ])

      [node] = graph["nodes"]
      assert node["properties"] == %{"status" => "customer"}
      assert node["created_at"] == "2026-01-01T00:00:00Z"
      assert node["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "node_type filter restricts nodes and edges" do
      org = "node:organization:o-1"
      person = "node:person:p-1"

      events = [
        ev(
          "prime.node.created",
          org,
          %{"node_type" => "organization", "properties" => %{}},
          "t0"
        ),
        ev("prime.node.created", person, %{"node_type" => "person", "properties" => %{}}, "t1"),
        ev(
          "prime.edge.created",
          "edge:e-1",
          %{"id" => "e-1", "source" => person, "target" => org, "relation" => "works_at"},
          "t2"
        )
      ]

      graph = GraphFold.fold(events, node_type: "organization")

      assert Enum.map(graph["nodes"], & &1["id"]) == [org]
      # edge crosses into a filtered-out node, so it is dropped
      assert graph["edges"] == []
      assert graph["stats"]["nodes_by_type"] == %{"organization" => 1}
    end

    test "limit paginates nodes and sets has_more" do
      events =
        for i <- 1..5 do
          ev(
            "prime.node.created",
            "node:person:p-#{i}",
            %{"node_type" => "person", "properties" => %{}},
            "t#{i}"
          )
        end

      graph = GraphFold.fold(events, limit: 3)

      assert length(graph["nodes"]) == 3
      assert graph["stats"]["node_count"] == 3
      # nodes_by_type reflects the full filtered set, not the page
      assert graph["stats"]["nodes_by_type"] == %{"person" => 5}
      assert graph["has_more"] == true
    end

    test "derives node_type from the node:<type>:<uuid> entity_id when payload omits it" do
      graph =
        GraphFold.fold([
          ev("prime.node.created", "node:project:proj-1", %{"properties" => %{}}, "t0")
        ])

      [node] = graph["nodes"]
      assert node["node_type"] == "project"
    end

    test "vector.deleted clears has_vector" do
      n = "node:doc:d-1"

      graph =
        GraphFold.fold([
          ev("prime.node.created", n, %{"node_type" => "doc", "properties" => %{}}, "t0"),
          ev("prime.vector.stored", "vec:" <> n, %{"dimensions" => 384}, "t1"),
          ev("prime.vector.deleted", "vec:" <> n, %{}, "t2")
        ])

      [node] = graph["nodes"]
      assert node["has_vector"] == false
      assert node["vector_dim"] == nil
      assert graph["stats"]["vector_count"] == 0
    end

    test "empty event list yields an empty but well-formed contract" do
      graph = GraphFold.fold([])

      assert graph == %{
               "nodes" => [],
               "edges" => [],
               "stats" => %{
                 "node_count" => 0,
                 "edge_count" => 0,
                 "vector_count" => 0,
                 "nodes_by_type" => %{}
               },
               "has_more" => false
             }
    end
  end
end
