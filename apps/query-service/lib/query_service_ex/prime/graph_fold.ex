defmodule QueryServiceEx.Prime.GraphFold do
  @moduledoc """
  Materializes the hosted Prime knowledge graph from a tenant's `prime.*`
  events in the MAIN (multi-tenant) event store.

  Synced tenant memory arrives as `prime.*` events (allsource-prime →
  control-plane → `POST /api/v1/events`) and lives **tenant-scoped in the main
  event store** — NOT in Core's single-tenant embedded Prime store. This module
  folds those events into the exact full-graph JSON contract the dashboard
  expects, so the hosted `GET /api/v1/prime/graph` returns a tenant's real graph
  instead of the empty embedded store.

  ## Contract (must match Core's embedded `FullGraph`)

      %{
        "nodes" => [%{"id" => "node:<type>:<uuid>", "node_type" => ...,
                      "properties" => ..., "has_vector" => bool,
                      "vector_dim" => int | nil, "created_at" => ...,
                      "updated_at" => ...}],
        "edges" => [%{"source" => "node:...", "target" => "node:...",
                      "relation" => ..., "properties" => ... | nil,
                      "weight" => float | nil, "created_at" => ...}],
        "stats" => %{"node_count" => N, "edge_count" => M,
                     "vector_count" => V, "nodes_by_type" => %{type => count}},
        "has_more" => bool
      }

  ## Verified event payload shapes (apps/core/src/prime)

    * `prime.node.created` / `prime.node.updated` —
      `entity_id = "node:<type>:<uuid>"`,
      `payload = {"id", "node_type", "properties"}`
    * `prime.node.deleted` — `payload = {"id"}` (entity_id is the node wire id)
    * `prime.edge.created` — `entity_id = "edge:<uuid>"`,
      `payload = {"id", "source", "target", "relation", "weight"?, "properties"?}`
      where `source`/`target` are node wire ids (`node:<type>:<uuid>`)
    * `prime.edge.deleted` — `payload = {"id"}` (entity_id is `edge:<uuid>`)
    * `prime.vector.stored` — `entity_id = "vec:<node-wire-id>"` (i.e.
      `vec:node:<type>:<uuid>`), `payload = {"text", "dimensions", "metadata"}`.
      Strip the `vec:` prefix to recover the node wire id it belongs to.
    * `prime.vector.deleted` — `entity_id = "vec:<node-wire-id>"`

  Events are folded in chronological order (Core returns ascending by
  `(timestamp, version)` by default), so created precedes updated/deleted.
  """

  @node_created "prime.node.created"
  @node_updated "prime.node.updated"
  @node_deleted "prime.node.deleted"
  @edge_created "prime.edge.created"
  @edge_deleted "prime.edge.deleted"
  @vector_stored "prime.vector.stored"
  @vector_deleted "prime.vector.deleted"

  @doc """
  Fold a chronological list of `prime.*` events into the full-graph contract.

  `opts`:
    * `:node_type` — keep only nodes of this type (and edges between them)
    * `:limit` — cap the node set; sets `has_more` when more nodes exist
  """
  def fold(events, opts \\ []) when is_list(events) do
    node_type_filter = opt_string(opts[:node_type])
    limit = opt_int(opts[:limit])

    state =
      Enum.reduce(events, %{nodes: %{}, edges: %{}, vectors: MapSet.new()}, &apply_event/2)

    # Live nodes, with deterministic ordering by wire id so pagination is stable
    # (mirrors Core's embedded sort_by id).
    all_nodes =
      state.nodes
      |> Map.values()
      |> maybe_filter_node_type(node_type_filter)
      |> Enum.sort_by(& &1.id)

    # nodes_by_type reflects the *unpaginated* filtered set (like Core).
    nodes_by_type =
      Enum.reduce(all_nodes, %{}, fn n, acc ->
        Map.update(acc, n.node_type, 1, &(&1 + 1))
      end)

    total_filtered = length(all_nodes)
    has_more = is_integer(limit) and total_filtered > limit

    nodes = if is_integer(limit), do: Enum.take(all_nodes, limit), else: all_nodes

    included = MapSet.new(nodes, & &1.id)

    vector_count = Enum.count(nodes, & &1.has_vector)

    # Edges restricted to the returned node set — drops edges whose source or
    # target node is absent (deleted, filtered, or paginated out), so the graph
    # is always internally consistent and a tenant/type filter can't leak edges.
    edges =
      state.edges
      |> Map.values()
      |> Enum.filter(fn e ->
        MapSet.member?(included, e.source) and MapSet.member?(included, e.target)
      end)
      |> Enum.sort_by(&{&1.source, &1.target, &1.relation})

    %{
      "nodes" => Enum.map(nodes, &node_to_wire/1),
      "edges" => Enum.map(edges, &edge_to_wire/1),
      "stats" => %{
        "node_count" => length(nodes),
        "edge_count" => length(edges),
        "vector_count" => vector_count,
        "nodes_by_type" => nodes_by_type
      },
      "has_more" => has_more
    }
  end

  # ---------------------------------------------------------------------------
  # Event application
  # ---------------------------------------------------------------------------

  defp apply_event(event, state) do
    type = field(event, "event_type")
    entity_id = field(event, "entity_id")
    payload = field(event, "payload") || %{}
    timestamp = field(event, "timestamp")

    case type do
      @node_created -> upsert_node(state, entity_id, payload, timestamp, :create)
      @node_updated -> upsert_node(state, entity_id, payload, timestamp, :update)
      @node_deleted -> drop_node(state, entity_id)
      @edge_created -> add_edge(state, entity_id, payload, timestamp)
      @edge_deleted -> drop_edge(state, entity_id, payload)
      @vector_stored -> mark_vector(state, entity_id, payload)
      @vector_deleted -> unmark_vector(state, entity_id)
      _ -> state
    end
  end

  defp upsert_node(state, entity_id, payload, timestamp, kind) do
    wire = node_wire_id(entity_id, payload)

    node_type =
      string_or_nil(payload["node_type"]) || node_type_from_wire(wire) || "unknown"

    properties = payload["properties"] || %{}

    nodes =
      Map.update(
        state.nodes,
        wire,
        %{
          id: wire,
          node_type: node_type,
          properties: properties,
          has_vector: false,
          vector_dim: nil,
          created_at: timestamp,
          updated_at: timestamp
        },
        fn existing ->
          %{
            existing
            | # latest node_type/properties win
              node_type: node_type,
              properties: properties,
              updated_at: timestamp || existing.updated_at,
              # first-seen created_at wins; a stray update before a create keeps
              # the earliest timestamp we have
              created_at:
                if(kind == :create,
                  do: existing.created_at || timestamp,
                  else: existing.created_at
                )
          }
        end
      )

    %{state | nodes: nodes}
  end

  defp drop_node(state, entity_id) do
    %{state | nodes: Map.delete(state.nodes, entity_id)}
  end

  defp add_edge(state, entity_id, payload, timestamp) do
    edge_id = string_or_nil(payload["id"]) || edge_id_from_entity(entity_id) || entity_id
    source = string_or_nil(payload["source"])
    target = string_or_nil(payload["target"])

    if is_nil(source) or is_nil(target) do
      state
    else
      edge = %{
        id: edge_id,
        source: source,
        target: target,
        relation: string_or_nil(payload["relation"]) || "related_to",
        properties: payload["properties"],
        weight: numeric_or_nil(payload["weight"]),
        created_at: timestamp
      }

      %{state | edges: Map.put(state.edges, edge_id, edge)}
    end
  end

  defp drop_edge(state, entity_id, payload) do
    edge_id = string_or_nil(payload["id"]) || edge_id_from_entity(entity_id) || entity_id
    %{state | edges: Map.delete(state.edges, edge_id)}
  end

  defp mark_vector(state, entity_id, payload) do
    node_wire = node_wire_from_vector_entity(entity_id)
    dim = vector_dim(payload)

    nodes =
      case Map.fetch(state.nodes, node_wire) do
        {:ok, node} ->
          Map.put(state.nodes, node_wire, %{node | has_vector: true, vector_dim: dim})

        :error ->
          # Vector arrived before/without its node; remember it so the node
          # picks up has_vector once it exists in this fold.
          state.nodes
      end

    %{state | nodes: nodes, vectors: MapSet.put(state.vectors, {node_wire, dim})}
  end

  defp unmark_vector(state, entity_id) do
    node_wire = node_wire_from_vector_entity(entity_id)

    nodes =
      case Map.fetch(state.nodes, node_wire) do
        {:ok, node} ->
          Map.put(state.nodes, node_wire, %{node | has_vector: false, vector_dim: nil})

        :error ->
          state.nodes
      end

    %{
      state
      | nodes: nodes,
        vectors: MapSet.reject(state.vectors, fn {w, _} -> w == node_wire end)
    }
  end

  # ---------------------------------------------------------------------------
  # Wire serialization
  # ---------------------------------------------------------------------------

  defp node_to_wire(n) do
    %{
      "id" => n.id,
      "node_type" => n.node_type,
      "properties" => n.properties,
      "has_vector" => n.has_vector,
      "vector_dim" => n.vector_dim,
      "created_at" => n.created_at,
      "updated_at" => n.updated_at
    }
  end

  defp edge_to_wire(e) do
    %{
      "source" => e.source,
      "target" => e.target,
      "relation" => e.relation,
      "properties" => e.properties,
      "weight" => e.weight,
      "created_at" => e.created_at
    }
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # The node's wire id is the event entity_id (`node:<type>:<uuid>`). Fall back
  # to reconstructing it from the payload if entity_id is somehow absent.
  defp node_wire_id(entity_id, payload) do
    cond do
      is_binary(entity_id) and String.starts_with?(entity_id, "node:") ->
        entity_id

      is_binary(entity_id) and entity_id != "" ->
        entity_id

      true ->
        type = string_or_nil(payload["node_type"]) || "unknown"
        id = string_or_nil(payload["id"]) || ""
        "node:#{type}:#{id}"
    end
  end

  # node:<type>:<uuid> -> <type>
  defp node_type_from_wire(wire) when is_binary(wire) do
    case String.split(wire, ":", parts: 3) do
      ["node", type, _id] when type != "" -> type
      _ -> nil
    end
  end

  defp node_type_from_wire(_), do: nil

  # vec:node:<type>:<uuid> -> node:<type>:<uuid>
  defp node_wire_from_vector_entity(entity_id) when is_binary(entity_id) do
    case entity_id do
      "vec:" <> rest -> rest
      other -> other
    end
  end

  defp node_wire_from_vector_entity(other), do: other

  # edge:<uuid> -> <uuid>
  defp edge_id_from_entity(entity_id) when is_binary(entity_id) do
    case entity_id do
      "edge:" <> rest -> rest
      _ -> nil
    end
  end

  defp edge_id_from_entity(_), do: nil

  defp vector_dim(payload) do
    case payload do
      %{"dimensions" => d} when is_integer(d) -> d
      %{"vector_dim" => d} when is_integer(d) -> d
      %{"vector" => v} when is_list(v) -> length(v)
      _ -> nil
    end
  end

  defp maybe_filter_node_type(nodes, nil), do: nodes
  defp maybe_filter_node_type(nodes, type), do: Enum.filter(nodes, &(&1.node_type == type))

  defp field(event, key) when is_map(event), do: event[key] || event[String.to_atom(key)]
  defp field(_event, _key), do: nil

  defp string_or_nil(v) when is_binary(v) and v != "", do: v
  defp string_or_nil(_), do: nil

  defp numeric_or_nil(v) when is_number(v), do: v
  defp numeric_or_nil(_), do: nil

  defp opt_string(nil), do: nil
  defp opt_string(""), do: nil
  defp opt_string(v) when is_binary(v), do: v
  defp opt_string(v), do: to_string(v)

  defp opt_int(nil), do: nil
  defp opt_int(v) when is_integer(v), do: v

  defp opt_int(v) when is_binary(v) do
    case Integer.parse(v) do
      {n, _} -> n
      :error -> nil
    end
  end

  defp opt_int(_), do: nil
end
