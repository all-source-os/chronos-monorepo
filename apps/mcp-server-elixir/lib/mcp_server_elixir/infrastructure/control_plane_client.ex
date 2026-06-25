defmodule McpServerElixir.Infrastructure.ControlPlaneClient do
  @moduledoc """
  HTTP client for communicating with the Go Control Plane API.

  Provides methods for cluster management and status monitoring.

  ## Route groups

  Two distinct route groups are reached from here:

  - **Non-admin tenant group** (`/api/v1/tenants/*`) — the original `tenant_*`
    functions. These do not require the admin JWT.
  - **Admin group** (`/api/v1/admin/*`) — the fleet-health + recovery functions
    (`fleet_health/2`, `fleet_tenant_health/3`, `recovery_*`). The Control Plane
    protects this group with `AdminAuthMiddleware`, which requires a
    `Authorization: Bearer <jwt>` carrying `role: "admin"`
    (`apps/control-plane/internal/interfaces/http/admin_middleware.go:45,70`).

  To let the admin-group calls through, `new/0` reads the admin JWT from the
  `ALLSOURCE_ADMIN_JWT` env var and a module-level header plug attaches it as a
  `Bearer` token on every request. The header is harmless on the non-admin
  tenant routes (they ignore it), so the existing `tenant_*` calls keep working.
  """

  use Tesla

  @default_base_url "http://localhost:3901"
  @default_timeout 10_000

  # Disable Hackney connection pooling — same rationale as CoreClient
  adapter(Tesla.Adapter.Hackney, pool: false, recv_timeout: @default_timeout)

  plug(
    Tesla.Middleware.BaseUrl,
    Application.get_env(:mcp_server_elixir, :control_url, @default_base_url)
  )

  # Attach the admin JWT (when configured) so the /api/v1/admin/* group's
  # AdminAuthMiddleware accepts fleet/recovery calls. The non-admin
  # /api/v1/tenants/* group ignores the header, so this is safe for all calls.
  plug(Tesla.Middleware.Headers, [{"authorization", "Bearer #{admin_jwt()}"}])

  plug(Tesla.Middleware.JSON)
  plug(Tesla.Middleware.Timeout, timeout: @default_timeout)

  plug(Tesla.Middleware.Retry,
    delay: 500,
    max_retries: 3,
    max_delay: 5_000,
    should_retry: fn
      {:ok, %{status: status}} when status in [408, 429, 500, 502, 503, 504] -> true
      {:ok, _} -> false
      {:error, _} -> true
    end
  )

  @doc """
  Build a client handle.

  Reads the admin JWT (`ALLSOURCE_ADMIN_JWT`) so callers can introspect whether
  the admin route group is reachable. The token itself is attached to outbound
  requests by the module-level `Tesla.Middleware.Headers` plug above.
  """
  def new do
    %{client: __MODULE__, admin_jwt: admin_jwt()}
  end

  # Admin JWT used to authenticate against the /api/v1/admin/* group.
  # Empty string when unset — admin-group calls then 401, which the handlers
  # surface verbatim.
  defp admin_jwt do
    System.get_env("ALLSOURCE_ADMIN_JWT") || ""
  end

  @doc "Get cluster status and health information"
  def get_cluster_status(_client) do
    case get("/api/v1/cluster/status") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Tenant Management Endpoints (non-admin group: /api/v1/tenants/*)
  # ============================================================================

  @doc "Create a new tenant with quotas and settings"
  def tenant_create(_client, params) do
    case post("/api/v1/tenants", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Update tenant settings and quotas"
  def tenant_update(_client, tenant_id, params) do
    case put("/api/v1/tenants/#{tenant_id}", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get usage statistics for a tenant"
  def tenant_usage(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/usage", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get or update quota configuration for a tenant"
  def tenant_quotas(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/quotas", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Suspend a tenant (soft disable)"
  def tenant_suspend(_client, tenant_id, params) do
    case post("/api/v1/tenants/#{tenant_id}/suspend", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Export all data for a tenant"
  def tenant_export(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/tenants/#{tenant_id}/export", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Fleet Health & Recovery Endpoints (ADMIN group: /api/v1/admin/*)
  #
  # These reach the AdminAuthMiddleware-protected route group built in the
  # Control Plane (GET /api/v1/admin/fleet/health[/:id], POST
  # /api/v1/admin/recovery/*). They are thin pass-throughs: the health model and
  # the dry-run / confirmation guards live entirely in the Control Plane. This
  # client recomputes NOTHING — it forwards params and returns the response.
  # ============================================================================

  @doc """
  Fleet-wide health aggregate (read).

  GET /api/v1/admin/fleet/health — returns per-tier counts + the worst-N tenants
  with their contributing reasons. Optional `tier` filter and `limit` (worst-N).
  """
  def fleet_health(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/admin/fleet/health", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Single-tenant health assessment (read).

  GET /api/v1/admin/fleet/health/:id — all signals + observed values + the tier
  each triggered, plus the tenant's subscription snapshot.
  """
  def fleet_tenant_health(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/admin/fleet/health/#{tenant_id}", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Re-trigger ingestion / reconcile counters for a tenant (Guarded).

  POST /api/v1/admin/recovery/:id/resync. `params` carries `dry_run` (and `reason`)
  straight through; the Control Plane enforces the guard.
  """
  def recovery_resync(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/resync", params)
  end

  @doc """
  Re-apply the tier's entitlements (Guarded).

  POST /api/v1/admin/recovery/:id/reconcile-subscription.
  """
  def recovery_reconcile_subscription(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/reconcile-subscription", params)
  end

  @doc """
  Resolve dunning / past-due drift for a tenant (Guarded).

  POST /api/v1/admin/recovery/:id/resolve-dunning.
  """
  def recovery_resolve_dunning(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/resolve-dunning", params)
  end

  @doc """
  Rotate a tenant's API keys (Destructive — confirm_token).

  POST /api/v1/admin/recovery/:id/rotate-keys.
  """
  def recovery_rotate_keys(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/rotate-keys", params)
  end

  @doc """
  Re-provision a tenant's metadata (Destructive — confirm_tenant_id).

  POST /api/v1/admin/recovery/:id/reprovision.
  """
  def recovery_reprovision(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/reprovision", params)
  end

  @doc """
  Restore a tenant from a snapshot / replay (Destructive — confirm_tenant_id + snapshot_id).

  POST /api/v1/admin/recovery/:id/restore.
  """
  def recovery_restore(_client, tenant_id, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/#{tenant_id}/restore", params)
  end

  @doc """
  Apply one Guarded action across many tenants (Destructive-bounded).

  POST /api/v1/admin/recovery/batch. `params` carries `filter`, `action`,
  `max_tenants`, `dry_run`, and `confirm_token` straight through; the Control
  Plane enforces the blast-radius cap and the count echo.
  """
  def recovery_batch(_client, params \\ %{}) do
    recovery_post("/api/v1/admin/recovery/batch", params)
  end

  @doc """
  Detect the edition trap (Safe — read).

  GET /api/v1/admin/recovery/diagnose/edition — returns the detection + the exact
  remediation command (operator-executed). No mutation.
  """
  def recovery_diagnose_edition(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/admin/recovery/diagnose/edition", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Tenant-360 + Comms Endpoints (ADMIN group: /api/v1/admin/*)
  #
  # Power-tool parity (ADMIN_TENANT_POWER_TOOL §9 Phase 8). These reach the SAME
  # AdminAuthMiddleware-protected route group as fleet/recovery. THEY LIVE HERE in
  # mcp-server-elixir, NEVER in prime-mcp: prime-mcp is single-tenant by design,
  # so per-tenant 360 + cross-tenant cohort comms cross a tenant boundary it does
  # not have. Like the recovery_* fns these are thin pass-throughs: the 360 is a
  # pure read composition (no scoring); the notice send forwards dry_run +
  # confirm_token straight through and the Control Plane enforces the cohort
  # blast-radius / confirm-token / opt-out / audit guards. This client recomputes
  # NOTHING.
  # ============================================================================

  @doc """
  Admin tenant detail (read).

  GET /api/v1/admin/tenants/:id — identity + members + API keys + subscription
  snapshot. One of the three reads `tenant_overview/3` composes.
  """
  def admin_tenant_detail(_client, tenant_id) do
    admin_get("/api/v1/admin/tenants/#{tenant_id}")
  end

  @doc """
  Admin tenant usage (read).

  GET /api/v1/admin/tenants/:id/usage — daily usage / counters. The second of the
  three reads `tenant_overview/3` composes. Optional `period` passed straight
  through as a query param.
  """
  def admin_tenant_usage(_client, tenant_id, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    admin_get("/api/v1/admin/tenants/#{tenant_id}/usage", query: query_params)
  end

  @doc """
  Per-tenant 360 (read composition).

  Fans out the three EXISTING admin reads server-side here and returns a single
  payload — exactly the `tenant_overview` shape §9 Pillar A describes:

    - `identity`     ← GET /api/v1/admin/tenants/:id        (identity/members/keys/subscription)
    - `health`       ← GET /api/v1/admin/fleet/health/:id   (signals + tier)
    - `usage`        ← GET /api/v1/admin/tenants/:id/usage   (daily usage)

  Every sub-read is wrapped in `{ok, value} | {error, reason}` under its own key
  so one missing/odd section never crashes the whole 360 — a partial overview is
  still useful (matches the admin "surface the real error, don't blank the page"
  rule). All scoring/tiers come from the Control Plane; this just composes reads.
  """
  def tenant_overview(client, tenant_id, params \\ %{}) do
    identity = admin_tenant_detail(client, tenant_id)
    health = fleet_tenant_health(client, tenant_id)
    usage = admin_tenant_usage(client, tenant_id, params)

    {:ok,
     %{
       "tenant_id" => tenant_id,
       "identity" => section_payload(identity),
       "health" => section_payload(health),
       "usage" => section_payload(usage)
     }}
  end

  # Normalise a sub-read into a 360 section: a successful body passes through; an
  # error becomes a `{error: ...}` marker so the operator sees WHICH section
  # failed (and why) instead of a blank panel. Composes only — decides nothing.
  defp section_payload({:ok, body}), do: body
  defp section_payload({:error, reason}), do: %{"error" => to_string_reason(reason)}

  defp to_string_reason(reason) when is_binary(reason), do: reason
  defp to_string_reason(reason), do: inspect(reason)

  @doc """
  Create an in-app notice for a tenant or cohort (Guarded — mutating).

  POST /api/v1/admin/notices. Matches prompt 037's `CreateNoticeRequest` shape:
  `{audience: {tenant_id} | {tier|plan|health_tier}, title, body, severity,
  expires_at?, dry_run, confirm_token?}`. For a COHORT send, `dry_run: true`
  returns a `would` preview + a `confirm_token`; echo that token back with
  `dry_run: false` to apply. This client forwards `dry_run` + `confirm_token`
  straight through; the Control Plane enforces the blast-radius cap, the
  confirm-token echo, opt-out, and the Core audit event.
  """
  def admin_create_notice(_client, params) do
    body =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case post("/api/v1/admin/notices", body) do
      {:ok, %Tesla.Env{status: status, body: resp}} when status in [200, 201] ->
        {:ok, resp}

      {:ok, %Tesla.Env{status: status, body: resp}} ->
        {:error, "HTTP #{status}: #{inspect(resp)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  List in-app notices (read).

  GET /api/v1/admin/notices?tenant_id= — admin view of notices. `tenant_id` is
  optional (omit for all tenants).
  """
  def admin_list_notices(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    admin_get("/api/v1/admin/notices", query: query_params)
  end

  # Shared GET helper for the admin-group reads (tenant detail/usage, notices).
  # Mirrors the inline read handling the other admin GETs use; centralised so the
  # 360 composition stays terse.
  defp admin_get(path, opts \\ []) do
    case get(path, opts) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # Shared POST helper for the mutating recovery endpoints. Passes the body
  # (dry_run + confirmation params + action-specific fields) straight through.
  defp recovery_post(path, params) do
    body =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case post(path, body) do
      {:ok, %Tesla.Env{status: status, body: resp}} when status in [200, 201] ->
        {:ok, resp}

      {:ok, %Tesla.Env{status: status, body: resp}} ->
        {:error, "HTTP #{status}: #{inspect(resp)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
