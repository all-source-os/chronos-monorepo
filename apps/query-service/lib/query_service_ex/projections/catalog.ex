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
        # entity_key/1 maps an event to the ETS entity dimension. Tenant-wide
        # counters use a single synthetic key ("_tenant"); per-entity templates
        # use the event's entity_id.
        entity_key: (event -> String.t()),
        initial: term(),
        reduce: (acc, event -> acc)
      }
  """

  @typedoc "A projection template definition."
  @type template :: %{
          name: String.t(),
          title: String.t(),
          description: String.t(),
          entity_key: (map() -> String.t()),
          initial: term(),
          reduce: (term(), map() -> term())
        }

  @tenant_key "_tenant"

  @doc """
  List all curated templates.
  """
  @spec list() :: [template()]
  def list, do: [event_count(), entity_activity()]

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

  # -- Templates --

  @doc false
  def event_count do
    %{
      name: "event-count",
      title: "Event Count",
      description: "Counts the tenant's events — a running total plus a breakdown by event_type.",
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
  def entity_activity do
    %{
      name: "entity-activity",
      title: "Entity Activity",
      description: "Per entity_id: number of events, when the last event arrived, and its type.",
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

  # Events may arrive with string or atom keys depending on source (Core JSON
  # vs internal). Read both.
  defp event_field(event, key) when is_map(event) do
    event[key] || event[String.to_existing_atom(key)]
  rescue
    ArgumentError -> event[key]
  end

  defp event_field(_event, _key), do: nil
end
