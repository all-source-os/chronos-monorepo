defmodule QueryServiceEx.Projections.Enablement do
  @moduledoc """
  Per-tenant projection enablement: the durable enabled set lives in Core tenant
  metadata at `metadata.projections.enabled` (an opaque JSON array of template
  names). Core stores it and never interprets it.

  This module is the QS-side authority for that set: it reads it via
  `RustCoreClient.get_tenant/1`, validates against the curated
  `QueryServiceEx.Projections.Catalog`, and writes it back via
  `RustCoreClient.merge_tenant_metadata/2` (a tenant-scoped atomic deep-merge that
  preserves sibling keys such as `metadata.quotas`).

  Enabling/disabling also drives the QS-owned folding engine
  (`QueryServiceEx.Projections.TenantProjections`): enable kicks off a background
  backfill (`building` → `ready`); disable drops the folded state.

  All operations are tenant-scoped and fail-closed at the controller boundary.
  """

  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.Projections.Catalog
  alias QueryServiceEx.Projections.TenantProjections

  # The Core client module is injectable so tests can supply a stub without HTTP.
  # Defaults to the real RustCoreClient. Must expose get_tenant/1 and
  # merge_tenant_metadata/2.
  defp core_client do
    Application.get_env(:query_service_ex, :projections_core_client, RustCoreClient)
  end

  @doc """
  Read a tenant's enabled set from Core metadata. Defaults to `[]`.

  Returns `{:ok, [name]}` or `{:error, reason}`.
  """
  @spec enabled_set(String.t()) :: {:ok, [String.t()]} | {:error, term()}
  def enabled_set(tenant_id) when is_binary(tenant_id) do
    case core_client().get_tenant(tenant_id) do
      {:ok, tenant} -> {:ok, read_enabled(tenant)}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  The tenant's enabled projections with live QS status, for the list endpoint.

  Returns `{:ok, [%{name, title, description, status}]}`. Status is
  `"building"`/`"ready"` from the folding engine, falling back to `"building"`
  for a name that is in the enabled set but has no QS status yet (e.g. after a
  QS restart, before re-fold).
  """
  @spec list(String.t()) :: {:ok, [map()]} | {:error, term()}
  def list(tenant_id) when is_binary(tenant_id) do
    with {:ok, enabled} <- enabled_set(tenant_id) do
      {:ok, Enum.map(enabled, &describe(tenant_id, &1))}
    end
  end

  @doc """
  Enable a template for a tenant.

  Validates the template name against the catalog, adds it to the enabled set
  (read-modify-write into Core metadata), and triggers a background backfill.

  Returns:
    * `{:ok, projection_map}` with `status: "building"`
    * `{:error, :unknown_template}`
    * `{:error, reason}` on a Core failure
  """
  @spec enable(String.t(), String.t()) :: {:ok, map()} | {:error, term()}
  def enable(tenant_id, template_name)
      when is_binary(tenant_id) and is_binary(template_name) do
    case Catalog.fetch(template_name) do
      {:ok, _template} ->
        with {:ok, enabled} <- enabled_set(tenant_id),
             new_enabled = enabled |> List.insert_at(-1, template_name) |> Enum.uniq(),
             {:ok, _} <- persist_enabled(tenant_id, new_enabled) do
          TenantProjections.enable(tenant_id, template_name)
          {:ok, describe(tenant_id, template_name)}
        end

      :error ->
        {:error, :unknown_template}
    end
  end

  @doc """
  Disable a projection for a tenant: remove it from the enabled set and drop its
  folded QS state.

  Returns `{:ok, name}` or `{:error, reason}`.
  """
  @spec disable(String.t(), String.t()) :: {:ok, String.t()} | {:error, term()}
  def disable(tenant_id, name) when is_binary(tenant_id) and is_binary(name) do
    with {:ok, enabled} <- enabled_set(tenant_id),
         new_enabled = Enum.reject(enabled, &(&1 == name)),
         {:ok, _} <- persist_enabled(tenant_id, new_enabled) do
      TenantProjections.disable(tenant_id, name)
      {:ok, name}
    end
  end

  # -- Internal --

  defp persist_enabled(tenant_id, enabled) do
    # Deep-merge: Core preserves metadata.quotas and other siblings.
    core_client().merge_tenant_metadata(tenant_id, %{
      "projections" => %{"enabled" => enabled}
    })
  end

  defp read_enabled(tenant) when is_map(tenant) do
    meta = tenant["metadata"] || tenant[:metadata] || %{}
    projections = meta["projections"] || meta[:projections] || %{}
    enabled = projections["enabled"] || projections[:enabled] || []

    enabled
    |> List.wrap()
    |> Enum.filter(&is_binary/1)
  end

  defp read_enabled(_), do: []

  defp describe(tenant_id, name) do
    status = TenantProjections.status(tenant_id, name) || :building

    base =
      case Catalog.fetch(name) do
        {:ok, t} -> %{name: t.name, title: t.title, description: t.description}
        :error -> %{name: name, title: name, description: nil}
      end

    Map.put(base, :status, Atom.to_string(status))
  end
end
