defmodule QueryServiceEx.Repo do
  use Ecto.Repo,
    otp_app: :query_service_ex,
    adapter: Ecto.Adapters.Postgres
end
