# Query Service Test Helper
#
# Tests require a PostgreSQL database running on localhost:5432.
# The Makefile handles starting/stopping the test database container.
#
# Manual testing:
#   docker run --name query-service-test-db -e POSTGRES_PASSWORD=postgres \
#     -e POSTGRES_DB=query_service_test -p 5432:5432 -d postgres:15
#   mix test
#   docker stop query-service-test-db

ExUnit.start()

Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
