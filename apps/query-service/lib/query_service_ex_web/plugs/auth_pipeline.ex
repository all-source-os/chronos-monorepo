defmodule QueryServiceExWeb.Plugs.AuthPipeline do
  @moduledoc """
  Guardian authentication pipeline for protected routes.

  This pipeline verifies JWT tokens from the Authorization header
  and loads the associated user resource.
  """

  use Guardian.Plug.Pipeline,
    otp_app: :query_service_ex,
    module: QueryServiceEx.Accounts.Guardian,
    error_handler: QueryServiceExWeb.Plugs.AuthErrorHandler

  plug(Guardian.Plug.VerifyHeader, claims: %{"typ" => "access"})
  plug(Guardian.Plug.EnsureAuthenticated)
  plug(Guardian.Plug.LoadResource)
end
