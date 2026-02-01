defmodule QueryServiceEx.UsageMeter.UsageRecord do
  @moduledoc """
  Schema for tracking individual usage records.

  Each record represents a single metered operation (event creation, query execution)
  and provides an audit trail for billing and analytics.
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  schema "usage_records" do
    field(:tenant_id, :binary_id)
    field(:usage_type, :string)
    field(:count, :integer)
    field(:metadata, :map, default: %{})
    field(:recorded_at, :utc_datetime)

    timestamps(type: :utc_datetime)
  end

  @doc """
  Changeset for creating a usage record.
  """
  def changeset(record, attrs) do
    record
    |> cast(attrs, [:tenant_id, :usage_type, :count, :metadata, :recorded_at])
    |> validate_required([:tenant_id, :usage_type, :count, :recorded_at])
    |> validate_inclusion(:usage_type, ["events", "queries"])
    |> validate_number(:count, greater_than: 0)
  end
end
