defmodule QueryServiceExWeb.IntegrationsControllerTest do
  use QueryServiceExWeb.ConnCase

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Tenants

  @moduletag :database

  describe "GET /api/integrations (all_status) - public" do
    test "returns status of all integrations", %{conn: conn} do
      conn = get(conn, "/api/integrations")
      response = json_response(conn, 200)

      assert Map.has_key?(response, "kafka")
      assert Map.has_key?(response, "rabbitmq")

      assert Map.has_key?(response["kafka"], "producer")
      assert Map.has_key?(response["kafka"], "consumer")
      assert Map.has_key?(response["kafka"], "enabled")

      assert Map.has_key?(response["rabbitmq"], "producer")
      assert Map.has_key?(response["rabbitmq"], "consumer")
      assert Map.has_key?(response["rabbitmq"], "enabled")
    end

    test "returns not_started status for producers", %{conn: conn} do
      conn = get(conn, "/api/integrations")
      response = json_response(conn, 200)

      # Producers aren't started in test environment
      assert response["kafka"]["producer"]["status"] == "not_started"
      assert response["rabbitmq"]["producer"]["status"] == "not_started"
    end
  end

  describe "GET /api/integrations/kafka (kafka_status) - public" do
    test "returns kafka integration status", %{conn: conn} do
      conn = get(conn, "/api/integrations/kafka")
      response = json_response(conn, 200)

      assert Map.has_key?(response, "producer")
      assert Map.has_key?(response, "consumer")
      assert Map.has_key?(response, "enabled")
    end

    test "returns producer status with expected fields", %{conn: conn} do
      conn = get(conn, "/api/integrations/kafka")
      response = json_response(conn, 200)

      assert Map.has_key?(response["producer"], "status")
    end

    test "returns consumer status with expected fields", %{conn: conn} do
      conn = get(conn, "/api/integrations/kafka")
      response = json_response(conn, 200)

      assert Map.has_key?(response["consumer"], "status")
    end
  end

  describe "GET /api/integrations/rabbitmq (rabbitmq_status) - public" do
    test "returns rabbitmq integration status", %{conn: conn} do
      conn = get(conn, "/api/integrations/rabbitmq")
      response = json_response(conn, 200)

      assert Map.has_key?(response, "producer")
      assert Map.has_key?(response, "consumer")
      assert Map.has_key?(response, "enabled")
    end

    test "returns producer status with expected fields", %{conn: conn} do
      conn = get(conn, "/api/integrations/rabbitmq")
      response = json_response(conn, 200)

      assert Map.has_key?(response["producer"], "status")
    end

    test "returns consumer status with expected fields", %{conn: conn} do
      conn = get(conn, "/api/integrations/rabbitmq")
      response = json_response(conn, 200)

      assert Map.has_key?(response["consumer"], "status")
    end
  end

  describe "GET /api/integrations/kafka/config (kafka_config) - authenticated" do
    setup %{conn: conn} do
      {:ok, conn: setup_auth(conn)}
    end

    test "returns kafka configuration", %{conn: conn} do
      conn = get(conn, "/api/integrations/kafka/config")
      response = json_response(conn, 200)

      assert Map.has_key?(response, "enabled")
      assert Map.has_key?(response, "status")
    end
  end

  describe "GET /api/integrations/rabbitmq/config (rabbitmq_config) - authenticated" do
    setup %{conn: conn} do
      {:ok, conn: setup_auth(conn)}
    end

    test "returns rabbitmq configuration", %{conn: conn} do
      conn = get(conn, "/api/integrations/rabbitmq/config")
      response = json_response(conn, 200)

      assert Map.has_key?(response, "enabled")
      assert Map.has_key?(response, "status")
    end
  end

  describe "POST /api/integrations/kafka/publish - authenticated" do
    setup %{conn: conn} do
      {:ok, conn: setup_auth(conn)}
    end

    test "returns error when kafka is disabled", %{conn: conn} do
      event = %{
        "event" => %{
          "entity_id" => "user-123",
          "event_type" => "user.created",
          "payload" => %{"email" => "test@example.com"}
        }
      }

      conn = post(conn, "/api/integrations/kafka/publish", event)
      response = json_response(conn, 422)

      assert response["error"] == "Kafka integration is disabled"
    end
  end

  describe "POST /api/integrations/rabbitmq/publish - authenticated" do
    setup %{conn: conn} do
      {:ok, conn: setup_auth(conn)}
    end

    test "returns error when rabbitmq is disabled", %{conn: conn} do
      event = %{
        "event" => %{
          "entity_id" => "user-123",
          "event_type" => "user.created",
          "payload" => %{"email" => "test@example.com"}
        }
      }

      conn = post(conn, "/api/integrations/rabbitmq/publish", event)
      response = json_response(conn, 422)

      assert response["error"] == "RabbitMQ integration is disabled"
    end
  end

  describe "authentication requirements" do
    test "kafka config requires authentication", %{conn: conn} do
      conn = get(conn, "/api/integrations/kafka/config")
      assert json_response(conn, 401)
    end

    test "rabbitmq config requires authentication", %{conn: conn} do
      conn = get(conn, "/api/integrations/rabbitmq/config")
      assert json_response(conn, 401)
    end

    test "kafka publish requires authentication", %{conn: conn} do
      conn = post(conn, "/api/integrations/kafka/publish", %{"event" => %{}})
      assert json_response(conn, 401)
    end

    test "rabbitmq publish requires authentication", %{conn: conn} do
      conn = post(conn, "/api/integrations/rabbitmq/publish", %{"event" => %{}})
      assert json_response(conn, 401)
    end

    test "status endpoints are public", %{conn: conn} do
      # These should not return 401
      assert get(conn, "/api/integrations") |> json_response(200)
      assert get(conn, "/api/integrations/kafka") |> json_response(200)
      assert get(conn, "/api/integrations/rabbitmq") |> json_response(200)
    end
  end

  # Helper to set up authentication
  defp setup_auth(conn) do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Test Workspace",
        slug: "test-workspace-#{:rand.uniform(100_000)}"
      })

    {:ok, user} =
      Accounts.create_user(%{
        email: "test-#{:rand.uniform(100_000)}@example.com",
        name: "Test User",
        provider: "google",
        google_id: "google_#{:rand.uniform(100_000)}",
        tenant_id: tenant.id
      })

    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    conn
    |> Plug.Conn.put_req_header("authorization", "Bearer #{token}")
    |> Plug.Conn.put_req_header("accept", "application/json")
  end
end
