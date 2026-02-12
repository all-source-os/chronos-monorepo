defmodule QueryServiceExWeb.Plugs.AuthPipeline do
  @moduledoc """
  Guardian authentication pipeline for protected routes.

  This pipeline verifies JWT tokens from the Authorization header
  and loads the associated user resource.

  When AUTH_DISABLED=true is set (development mode), authentication
  is bypassed and a dev user context is injected instead.
  """

  @behaviour Plug

  alias Guardian.Plug.{EnsureAuthenticated, LoadResource, VerifyHeader}
  alias QueryServiceEx.DevMode

  # Guardian module for this app
  @guardian_module QueryServiceEx.Accounts.Guardian
  @error_handler QueryServiceExWeb.Plugs.AuthErrorHandler

  def init(opts), do: opts

  def call(conn, _opts) do
    if DevMode.auth_disabled?() do
      # Dev mode: bypass authentication entirely
      bypass_auth(conn)
    else
      # Normal mode: run Guardian authentication pipeline
      run_guardian_pipeline(conn)
    end
  end

  # Bypass authentication and inject dev user/tenant context
  defp bypass_auth(conn) do
    dev_user = DevMode.dev_user()

    conn
    |> Plug.Conn.assign(:current_user, dev_user)
    |> Plug.Conn.put_private(:guardian_default_resource, dev_user)
    |> Plug.Conn.put_private(:chronos_dev_mode, true)
  end

  # Run the standard Guardian authentication pipeline
  defp run_guardian_pipeline(conn) do
    verify_opts = [
      module: @guardian_module,
      error_handler: @error_handler,
      claims: %{"typ" => "access"}
    ]

    ensure_opts = [
      module: @guardian_module,
      error_handler: @error_handler
    ]

    load_opts = [
      module: @guardian_module,
      error_handler: @error_handler
    ]

    conn
    |> VerifyHeader.call(VerifyHeader.init(verify_opts))
    |> EnsureAuthenticated.call(EnsureAuthenticated.init(ensure_opts))
    |> maybe_load_resource(load_opts)
  end

  defp maybe_load_resource(%Plug.Conn{halted: true} = conn, _opts), do: conn

  defp maybe_load_resource(conn, opts) do
    LoadResource.call(conn, LoadResource.init(opts))
  end
end
