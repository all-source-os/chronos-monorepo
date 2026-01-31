# Exclude database tests by default unless database is available
# Run with: mix test --include database
ExUnit.start(exclude: [:database])

# Only set up sandbox if we're including database tests
if :database not in (ExUnit.configuration()[:exclude] || []) do
  Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
end
