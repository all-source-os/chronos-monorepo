defmodule QueryServiceEx.Projections.Catalog do
  @moduledoc """
  Curated catalog of per-tenant projection *templates*.

  A template is a fold definition the Query Service applies to a tenant's
  (already isolated) event stream to maintain a per-tenant read-model. Templates
  are platform-defined and read-only — users do NOT author reducer code in v1
  (see `docs/proposals/PER_TENANT_PROJECTIONS.md`).

  These are **not** Core engine projections. Core's internal projections
  (`entity_snapshots`, `event_counters`, Prime's projections, the embedded demo
  sagas/portfolios/trades) are never templates.

  ## Template shape

      %{
        name: "event-count",
        title: "Event Count",
        description: "...",
        # kind is a render hint the dashboard switches on to format folded state
        # WITHOUT knowing the template name (see Gap 1 in the projections epic):
        #   :counter      — single headline number + optional by-key breakdown
        #   :breakdown    — label -> count map, rendered as a sorted table / bars
        #   :timeseries   — UTC-day -> count map, rendered as a compact chart
        #   :entity_table — per-entity rows, rendered as a small table
        kind: :counter | :breakdown | :timeseries | :entity_table,
        # entity_key/1 maps an event to the ETS entity dimension. Tenant-wide
        # counters/timeseries use a single synthetic key ("_tenant"); per-entity
        # templates use the event's entity_id.
        entity_key: (event -> String.t()),
        initial: term(),
        reduce: (acc, event -> acc)
      }

  ## Bounded folds

  Every reducer is a cheap pure fold that runs on every event during backfill and
  live folding. Folds that retain a growing collection (timeseries buckets,
  breakdown keys, entity rows) are **bounded**: they cap the retained data so a
  high-volume tenant cannot blow up QS memory. See `@timeseries_max_buckets` and
  `@entity_summary_max_rows`.
  """

  @typedoc "Render hint for the dashboard state view."
  @type kind :: :counter | :breakdown | :timeseries | :entity_table

  @typedoc "A projection template definition."
  @type template :: %{
          name: String.t(),
          title: String.t(),
          description: String.t(),
          kind: kind(),
          entity_key: (map() -> String.t()),
          initial: term(),
          reduce: (term(), map() -> term())
        }

  @tenant_key "_tenant"

  # Retain at most this many day-buckets in events-per-day (~3 months). Bounds
  # the timeseries map so a long-lived high-volume tenant can't grow it without
  # limit; the oldest buckets are dropped first.
  @timeseries_max_buckets 90

  # Retain at most this many rows in tenant-wide entity-summary projections
  # (active-entities most-recent list). Per-entity templates (one ETS row per
  # entity) are bounded by the natural entity cardinality, not here.
  @entity_summary_max_rows 50

  @doc """
  List all curated templates.
  """
  @spec list() :: [template()]
  def list,
    do: [
      event_count(),
      event_type_leaderboard(),
      events_per_day(),
      entity_activity(),
      active_entities()
    ]

  @doc """
  Fetch a template by name.

  Returns `{:ok, template}` or `:error` for unknown names.
  """
  @spec fetch(String.t()) :: {:ok, template()} | :error
  def fetch(name) when is_binary(name) do
    case Enum.find(list(), &(&1.name == name)) do
      nil -> :error
      template -> {:ok, template}
    end
  end

  def fetch(_), do: :error

  @doc """
  True if `name` is a known template name.
  """
  @spec valid?(String.t()) :: boolean()
  def valid?(name), do: match?({:ok, _}, fetch(name))

  @doc """
  The synthetic entity key used by tenant-wide (single-bucket) templates.
  """
  @spec tenant_key() :: String.t()
  def tenant_key, do: @tenant_key

  @doc """
  Max retained day-buckets for the events-per-day timeseries (exported for tests).
  """
  @spec timeseries_max_buckets() :: pos_integer()
  def timeseries_max_buckets, do: @timeseries_max_buckets

  @doc """
  Max retained rows for tenant-wide entity-summary projections (exported for tests).
  """
  @spec entity_summary_max_rows() :: pos_integer()
  def entity_summary_max_rows, do: @entity_summary_max_rows

  # -- Templates --

  @doc false
  def event_count do
    %{
      name: "event-count",
      title: "Event Count",
      description: "Counts the tenant's events — a running total plus a breakdown by event_type.",
      kind: :counter,
      entity_key: fn _event -> @tenant_key end,
      initial: %{"total" => 0, "by_event_type" => %{}},
      reduce: fn acc, event ->
        type = event_field(event, "event_type") || "unknown"
        by_type = Map.get(acc, "by_event_type", %{})

        acc
        |> Map.put("total", Map.get(acc, "total", 0) + 1)
        |> Map.put("by_event_type", Map.update(by_type, type, 1, &(&1 + 1)))
      end
    }
  end

  @doc false
  def event_type_leaderboard do
    %{
      name: "event-type-leaderboard",
      title: "Event Type Leaderboard",
      description: "Event types ranked by count — which kinds of events dominate your stream.",
      kind: :breakdown,
      entity_key: fn _event -> @tenant_key end,
      initial: %{"by_event_type" => %{}},
      reduce: fn acc, event ->
        type = event_field(event, "event_type") || "unknown"
        by_type = Map.get(acc, "by_event_type", %{})
        Map.put(acc, "by_event_type", Map.update(by_type, type, 1, &(&1 + 1)))
      end
    }
  end

  @doc false
  def events_per_day do
    %{
      name: "events-per-day",
      title: "Events Per Day",
      description: "Daily event volume — a timeseries bucketed by UTC day (last 90 days).",
      kind: :timeseries,
      entity_key: fn _event -> @tenant_key end,
      initial: %{"by_day" => %{}},
      reduce: fn acc, event ->
        day = utc_day(event_field(event, "timestamp"))
        by_day = Map.get(acc, "by_day", %{})

        by_day =
          if day do
            Map.update(by_day, day, 1, &(&1 + 1))
          else
            by_day
          end

        Map.put(acc, "by_day", cap_buckets(by_day, @timeseries_max_buckets))
      end
    }
  end

  @doc false
  def entity_activity do
    %{
      name: "entity-activity",
      title: "Entity Activity",
      description: "Per entity_id: number of events, when the last event arrived, and its type.",
      kind: :entity_table,
      entity_key: fn event -> event_field(event, "entity_id") || @tenant_key end,
      initial: %{"event_count" => 0, "last_event_at" => nil, "last_event_type" => nil},
      reduce: fn acc, event ->
        %{
          "event_count" => Map.get(acc, "event_count", 0) + 1,
          "last_event_at" => event_field(event, "timestamp") || acc["last_event_at"],
          "last_event_type" => event_field(event, "event_type") || acc["last_event_type"]
        }
      end
    }
  end

  @doc false
  def active_entities do
    %{
      name: "active-entities",
      title: "Active Entities",
      description:
        "Distinct entity count plus the most-recently-active entities (entity_id, last_event_at).",
      kind: :entity_table,
      entity_key: fn _event -> @tenant_key end,
      initial: %{"distinct" => 0, "recent" => %{}},
      reduce: fn acc, event ->
        entity = event_field(event, "entity_id")
        ts = event_field(event, "timestamp")
        recent = Map.get(acc, "recent", %{})

        cond do
          is_nil(entity) ->
            acc

          true ->
            new_entity? = not Map.has_key?(recent, entity)
            recent = Map.put(recent, entity, ts || recent[entity])
            recent = cap_recent(recent, @entity_summary_max_rows)

            distinct = Map.get(acc, "distinct", 0) + if(new_entity?, do: 1, else: 0)

            acc
            |> Map.put("distinct", distinct)
            |> Map.put("recent", recent)
        end
      end
    }
  end

  # -- Internal --

  # Keep only the `max` newest day-buckets (lexical date sort == chronological
  # for ISO YYYY-MM-DD keys). Bounds the timeseries map.
  defp cap_buckets(by_day, max) when map_size(by_day) <= max, do: by_day

  defp cap_buckets(by_day, max) do
    by_day
    |> Enum.sort_by(fn {day, _} -> day end, :desc)
    |> Enum.take(max)
    |> Map.new()
  end

  # Keep only the `max` most-recently-active entities (highest last_event_at
  # first). Bounds the active-entities recent map.
  defp cap_recent(recent, max) when map_size(recent) <= max, do: recent

  defp cap_recent(recent, max) do
    recent
    |> Enum.sort_by(fn {_id, ts} -> ts || "" end, :desc)
    |> Enum.take(max)
    |> Map.new()
  end

  # Bucket an ISO-8601 timestamp into its UTC day ("YYYY-MM-DD"). Returns nil for
  # an unparseable/absent timestamp so the event simply isn't bucketed.
  defp utc_day(nil), do: nil

  defp utc_day(ts) when is_binary(ts) do
    case String.split(ts, "T", parts: 2) do
      [date | _] when byte_size(date) >= 10 -> binary_part(date, 0, 10)
      _ -> nil
    end
  end

  defp utc_day(_), do: nil

  # Events may arrive with string or atom keys depending on source (Core JSON
  # vs internal). Read both.
  defp event_field(event, key) when is_map(event) do
    event[key] || event[String.to_existing_atom(key)]
  rescue
    ArgumentError -> event[key]
  end

  defp event_field(_event, _key), do: nil
end
