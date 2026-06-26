defmodule QueryServiceEx.Projections.CatalogTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.Catalog

  describe "list/0 and fetch/1" do
    test "lists the curated templates" do
      names = Catalog.list() |> Enum.map(& &1.name) |> Enum.sort()
      assert names == ["entity-activity", "event-count"]
    end

    test "fetch returns a known template" do
      assert {:ok, template} = Catalog.fetch("event-count")
      assert template.name == "event-count"
      assert is_function(template.reduce, 2)
      assert is_function(template.entity_key, 1)
    end

    test "fetch returns :error for unknown" do
      assert :error = Catalog.fetch("nope")
      assert :error = Catalog.fetch(nil)
    end

    test "valid?/1" do
      assert Catalog.valid?("entity-activity")
      refute Catalog.valid?("sagas")
    end
  end

  describe "event-count reducer" do
    test "counts total and by event_type into a single tenant bucket" do
      {:ok, t} = Catalog.fetch("event-count")

      events = [
        %{"event_type" => "user.created", "entity_id" => "u1"},
        %{"event_type" => "user.created", "entity_id" => "u2"},
        %{"event_type" => "order.placed", "entity_id" => "o1"}
      ]

      # all events share the synthetic tenant key
      assert Enum.all?(events, fn e -> t.entity_key.(e) == Catalog.tenant_key() end)

      final = Enum.reduce(events, t.initial, fn e, acc -> t.reduce.(acc, e) end)

      assert final["total"] == 3
      assert final["by_event_type"] == %{"user.created" => 2, "order.placed" => 1}
    end

    test "handles atom-keyed events" do
      {:ok, t} = Catalog.fetch("event-count")
      final = t.reduce.(t.initial, %{event_type: "a.b"})
      assert final["total"] == 1
      assert final["by_event_type"] == %{"a.b" => 1}
    end
  end

  describe "entity-activity reducer" do
    test "tracks per-entity count, last_event_at, last_event_type" do
      {:ok, t} = Catalog.fetch("entity-activity")

      e1 = %{"event_type" => "a", "entity_id" => "x", "timestamp" => "2026-01-01T00:00:00Z"}
      e2 = %{"event_type" => "b", "entity_id" => "x", "timestamp" => "2026-01-02T00:00:00Z"}

      assert t.entity_key.(e1) == "x"

      final =
        [e1, e2]
        |> Enum.reduce(t.initial, fn e, acc -> t.reduce.(acc, e) end)

      assert final["event_count"] == 2
      assert final["last_event_at"] == "2026-01-02T00:00:00Z"
      assert final["last_event_type"] == "b"
    end
  end
end
