defmodule QueryServiceExWeb.OnboardingControllerTest do
  @moduledoc """
  Tests for OnboardingController.
  """
  use QueryServiceExWeb.ConnCase

  import Plug.Conn

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Onboarding
  alias QueryServiceEx.Tenants

  @moduletag :database

  setup %{conn: conn} do
    # Create a tenant and user for testing
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Test Workspace",
        slug: "test-workspace-#{:rand.uniform(10000)}"
      })

    {:ok, user} =
      Accounts.create_user(%{
        email: "test#{:rand.uniform(10000)}@example.com",
        name: "Test User",
        provider: "google",
        google_id: "google_#{:rand.uniform(100_000)}",
        tenant_id: tenant.id
      })

    user = QueryServiceEx.Repo.preload(user, :tenant)

    # Create a valid JWT token
    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    conn =
      conn
      |> put_req_header("authorization", "Bearer #{token}")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn, tenant: tenant, user: user}
  end

  describe "GET /api/onboarding" do
    test "returns onboarding status for new user", %{conn: conn, tenant: tenant} do
      conn = get(conn, "/api/onboarding")
      response = json_response(conn, 200)["data"]

      assert response["tenant_id"] == tenant.id
      assert response["completed"] == false
      assert response["skipped"] == false
      assert response["current_step"] == "workspace_setup"
      assert response["completion_percentage"] == 0
      assert length(response["steps"]) == 5
      assert response["next_action"] != nil
    end

    test "returns status with completed steps", %{conn: conn, tenant: tenant} do
      {:ok, _} = Onboarding.complete_step(tenant.id, :workspace_setup)
      {:ok, _} = Onboarding.complete_step(tenant.id, :first_event)

      conn = get(conn, "/api/onboarding")
      response = json_response(conn, 200)["data"]

      assert response["completion_percentage"] == 40
      assert response["current_step"] == "first_query"

      completed_steps =
        response["steps"]
        |> Enum.filter(& &1["completed"])
        |> Enum.map(& &1["id"])

      assert "workspace_setup" in completed_steps
      assert "first_event" in completed_steps
    end

    test "returns 401 without auth token", %{conn: conn} do
      conn =
        conn
        |> delete_req_header("authorization")
        |> get("/api/onboarding")

      assert json_response(conn, 401)
    end
  end

  describe "GET /api/onboarding/steps" do
    test "returns all step details", %{conn: conn} do
      conn = get(conn, "/api/onboarding/steps")
      response = json_response(conn, 200)["data"]

      assert is_list(response)
      assert length(response) > 0

      step = Enum.find(response, &(&1["id"] == "workspace_setup"))
      assert step["title"] != nil
      assert step["description"] != nil
      assert step["docs_url"] != nil
    end
  end

  describe "POST /api/onboarding/steps/:step/complete" do
    test "completes a valid step", %{conn: conn} do
      conn = post(conn, "/api/onboarding/steps/workspace_setup/complete")
      response = json_response(conn, 200)["data"]

      completed_steps =
        response["steps"]
        |> Enum.filter(& &1["completed"])
        |> Enum.map(& &1["id"])

      assert "workspace_setup" in completed_steps
      assert response["current_step"] == "first_event"
    end

    test "completing all steps marks onboarding complete", %{conn: conn} do
      steps = ["workspace_setup", "first_event", "first_query", "first_projection", "invite_team"]

      for step <- steps do
        post(conn, "/api/onboarding/steps/#{step}/complete")
      end

      conn = get(conn, "/api/onboarding")
      response = json_response(conn, 200)["data"]

      assert response["completed"] == true
      assert response["completion_percentage"] == 100
    end

    test "returns error for invalid step", %{conn: conn} do
      conn = post(conn, "/api/onboarding/steps/invalid_step/complete")
      response = json_response(conn, 400)

      assert response["error"]["code"] == "invalid_step"
      assert is_list(response["error"]["valid_steps"])
    end
  end

  describe "POST /api/onboarding/skip" do
    test "skips onboarding", %{conn: conn} do
      conn = post(conn, "/api/onboarding/skip")
      response = json_response(conn, 200)["data"]

      assert response["skipped"] == true
      assert response["skipped_at"] != nil
    end
  end

  describe "POST /api/onboarding/reset" do
    test "resets onboarding progress", %{conn: conn, tenant: tenant} do
      # Complete some steps first
      {:ok, _} = Onboarding.complete_step(tenant.id, :workspace_setup)
      {:ok, _} = Onboarding.complete_step(tenant.id, :first_event)

      conn = post(conn, "/api/onboarding/reset")
      response = json_response(conn, 200)["data"]

      assert response["completed"] == false
      assert response["completion_percentage"] == 0
      assert response["current_step"] == "workspace_setup"

      completed_steps =
        response["steps"]
        |> Enum.filter(& &1["completed"])

      assert Enum.empty?(completed_steps)
    end

    test "resets skipped onboarding", %{conn: conn, tenant: tenant} do
      {:ok, _} = Onboarding.skip_onboarding(tenant.id)

      conn = post(conn, "/api/onboarding/reset")
      response = json_response(conn, 200)["data"]

      assert response["skipped"] == false
      assert response["skipped_at"] == nil
    end
  end
end
