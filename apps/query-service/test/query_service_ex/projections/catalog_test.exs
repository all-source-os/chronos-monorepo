defmodule QueryServiceEx.Projections.CatalogTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.Catalog

  describe "list/0 and fetch/1" do
    test "lists the curated templates" do
      names = Catalog.list() |> Enum.map(& &1.name) |> Enum.sort()

      assert names == [
               "active-entities",
               "entity-activity",
               "event-count",
               "event-type-leaderboard",
               "events-per-day"
             ]
    end

    test "every template carries a known render-hint kind" do
      valid_kinds = [:counter, :breakdown, :timeseries, :entity_table]

      for t <- Catalog.list() do
        assert t.kind in valid_kinds, "#{t.name} has invalid kind #{inspect(t.kind)}"
      end
    end

    test "fetch returns a known template" do
      assert {:ok, template} = Catalog.fetch("event-count")
      assert template.name == "event-count"
      assert template.kind == :counter
      assert is_function(template.reduce, 2)
      assert is_function(template.entity_key, 1)
    end

    test "fetch returns :error for unknown" do
      assert :error = Catalog.fetch("nope")
      assert :error = Catalog.fetch(nil)
    end

    test "valid?/1" do
      assert Catalog.valid?("entity-activity")
      assert Catalog.valid?("events-per-day")
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

  describe "event-type-leaderboard reducer" do
    test "counts events per type into a single tenant bucket" do
      {:ok, t} = Catalog.fetch("event-type-leaderboard")
      assert t.kind == :breakdown

      events = [
        %{"event_type" => "a"},
        %{"event_type" => "a"},
        %{"event_type" => "b"},
        %{"entity_id" => "x"}
      ]

      assert Enum.all?(events, fn e -> t.entity_key.(e) == Catalog.tenant_key() end)
      final = Enum.reduce(events, t.initial, fn e, acc -> t.reduce.(acc, e) end)
      assert final["by_event_type"] == %{"a" => 2, "b" => 1, "unknown" => 1}
    end
  end

  describe "events-per-day reducer" do
    test "buckets events by UTC day" do
      {:ok, t} = Catalog.fetch("events-per-day")
      assert t.kind == :timeseries

      events = [
        %{"timestamp" => "2026-01-01T03:00:00Z"},
        %{"timestamp" => "2026-01-01T23:59:59Z"},
        %{"timestamp" => "2026-01-02T00:00:01Z"}
      ]

      final = Enum.reduce(events, t.initial, fn e, acc -> t.reduce.(acc, e) end)
      assert final["by_day"] == %{"2026-01-01" => 2, "2026-01-02" => 1}
    end

    test "events without a parseable timestamp are not bucketed" do
      {:ok, t} = Catalog.fetch("events-per-day")
      final = t.reduce.(t.initial, %{"event_type" => "x"})
      assert final["by_day"] == %{}
    end

    test "caps retained day-buckets (bounded memory)" do
      {:ok, t} = Catalog.fetch("events-per-day")
      cap = Catalog.timeseries_max_buckets()

      # Seed cap + 30 distinct days; the map must never exceed the cap, and the
      # NEWEST days must be the ones retained (oldest dropped first).
      final =
        Enum.reduce(1..(cap + 30), t.initial, fn n, acc ->
          day = Date.add(~D[2026-01-01], n) |> Date.to_iso8601()
          t.reduce.(acc, %{"timestamp" => day <> "T00:00:00Z"})
        end)

      buckets = final["by_day"]
      assert map_size(buckets) == cap

      newest = Date.add(~D[2026-01-01], cap + 30) |> Date.to_iso8601()
      oldest_kept = Date.add(~D[2026-01-01], 31) |> Date.to_iso8601()
      assert Map.has_key?(buckets, newest)
      assert Map.has_key?(buckets, oldest_kept)
      # the very first (oldest) day was dropped
      refute Map.has_key?(buckets, "2026-01-02")
    end
  end

  describe "active-entities reducer" do
    test "tracks distinct count and most-recent entities" do
      {:ok, t} = Catalog.fetch("active-entities")
      assert t.kind == :entity_table
      assert t.entity_key.(%{"entity_id" => "x"}) == Catalog.tenant_key()

      events = [
        %{"entity_id" => "a", "timestamp" => "2026-01-01T00:00:00Z"},
        %{"entity_id" => "b", "timestamp" => "2026-01-02T00:00:00Z"},
        %{"entity_id" => "a", "timestamp" => "2026-01-03T00:00:00Z"}
      ]

      final = Enum.reduce(events, t.initial, fn e, acc -> t.reduce.(acc, e) end)
      assert final["distinct"] == 2
      assert final["recent"] == %{"a" => "2026-01-03T00:00:00Z", "b" => "2026-01-02T00:00:00Z"}
    end

    test "ignores events without an entity_id" do
      {:ok, t} = Catalog.fetch("active-entities")
      final = t.reduce.(t.initial, %{"event_type" => "x", "timestamp" => "2026-01-01T00:00:00Z"})
      assert final["distinct"] == 0
      assert final["recent"] == %{}
    end

    test "caps the recent-entities map (bounded memory) while distinct keeps counting" do
      {:ok, t} = Catalog.fetch("active-entities")
      cap = Catalog.entity_summary_max_rows()
      n = cap + 25

      final =
        Enum.reduce(1..n, t.initial, fn i, acc ->
          day = Date.add(~D[2026-01-01], i) |> Date.to_iso8601()
          t.reduce.(acc, %{"entity_id" => "e#{i}", "timestamp" => day <> "T00:00:00Z"})
        end)

      # distinct counts every entity; the recent map is bounded.
      assert final["distinct"] == n
      assert map_size(final["recent"]) == cap

      # the newest entity is retained, the oldest is dropped.
      assert Map.has_key?(final["recent"], "e#{n}")
      refute Map.has_key?(final["recent"], "e1")
    end
  end
end
