# Try to start the application (may fail if some components can't start in test env)
# Individual test setups will ensure required GenServers are running
try do
  Application.ensure_all_started(:mcp_server_elixir)
rescue
  _ -> :ok
catch
  _, _ -> :ok
end

ExUnit.start()
