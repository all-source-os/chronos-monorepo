defmodule QueryServiceExWeb.WebhookControllerTest do
  @moduledoc """
  Tests for WebhookController LemonSqueezy webhooks.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceExWeb.ConnCase

  alias QueryServiceEx.Tenants

  @moduletag :database

  @webhook_secret "test_webhook_secret_for_testing"

  setup %{conn: conn} do
    {:ok, tenant} =
      Tenants.create_tenant(%{name: "Webhook Test Workspace", slug: "webhook-test"})

    conn =
      conn
      |> put_req_header("content-type", "application/json")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn, tenant: tenant}
  end

  describe "POST /api/webhooks/lemonsqueezy" do
    test "returns unauthorized for invalid signature", %{conn: conn} do
      payload = subscription_created_payload("tenant-123")
      body = Jason.encode!(payload)

      conn =
        conn
        |> put_req_header("x-signature", "invalid_signature")
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 401)["error"] == "Invalid signature"
    end

    test "returns unauthorized when signature missing", %{conn: conn} do
      payload = subscription_created_payload("tenant-123")

      conn = post(conn, "/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 401)["error"] == "Invalid signature"
    end

    test "handles subscription_created event", %{conn: conn, tenant: tenant} do
      payload = subscription_created_payload(tenant.id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.lemon_squeezy_customer_id == "12345"
      assert updated_tenant.lemon_squeezy_subscription_id == "sub_67890"
      assert updated_tenant.subscription_status == :active
    end

    test "handles subscription_updated event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :active
        })

      payload = subscription_updated_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.subscription_status == :active
    end

    test "handles subscription_cancelled event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :active
        })

      payload = subscription_cancelled_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.subscription_status == :cancelled
    end

    test "handles subscription_resumed event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :cancelled
        })

      payload = subscription_resumed_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.subscription_status == :active
    end

    test "handles subscription_expired event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :cancelled
        })

      payload = subscription_expired_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.subscription_status == :expired
    end

    test "handles subscription_payment_success event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :active
        })

      {:ok, _} = Tenants.increment_events_usage(tenant.id, 5000)
      {:ok, _} = Tenants.increment_queries_usage(tenant.id, 500)

      payload = subscription_payment_success_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.events_used == 0
      assert updated_tenant.queries_used == 0
    end

    test "handles subscription_payment_failed event", %{conn: conn, tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_subscription_id: "sub_67890",
          subscription_status: :active
        })

      payload = subscription_payment_failed_payload(tenant.lemon_squeezy_subscription_id)
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true

      updated_tenant = Tenants.get_tenant!(tenant.id)
      assert updated_tenant.subscription_status == :past_due
    end

    test "ignores unknown event types", %{conn: conn} do
      payload = %{
        "meta" => %{"event_name" => "unknown_event"},
        "data" => %{"id" => "123", "attributes" => %{}}
      }

      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end
  end

  # Helpers

  defp compute_signature(body) do
    :crypto.mac(:hmac, :sha256, @webhook_secret, body)
    |> Base.encode16(case: :lower)
  end

  defp put_raw_body(conn, body) do
    Plug.Conn.assign(conn, :raw_body, body)
  end

  defp subscription_created_payload(tenant_id) do
    %{
      "meta" => %{"event_name" => "subscription_created"},
      "data" => %{
        "id" => "sub_67890",
        "attributes" => %{
          "customer_id" => 12345,
          "status" => "active",
          "variant_id" => "variant_starter",
          "custom_data" => %{"tenant_id" => tenant_id}
        }
      }
    }
  end

  defp subscription_updated_payload(subscription_id) do
    %{
      "meta" => %{"event_name" => "subscription_updated"},
      "data" => %{
        "id" => subscription_id,
        "attributes" => %{
          "status" => "active",
          "variant_id" => "variant_pro",
          "ends_at" => nil
        }
      }
    }
  end

  defp subscription_cancelled_payload(subscription_id) do
    ends_at = DateTime.utc_now() |> DateTime.add(30, :day) |> DateTime.to_iso8601()

    %{
      "meta" => %{"event_name" => "subscription_cancelled"},
      "data" => %{
        "id" => subscription_id,
        "attributes" => %{
          "ends_at" => ends_at
        }
      }
    }
  end

  defp subscription_resumed_payload(subscription_id) do
    %{
      "meta" => %{"event_name" => "subscription_resumed"},
      "data" => %{
        "id" => subscription_id,
        "attributes" => %{}
      }
    }
  end

  defp subscription_expired_payload(subscription_id) do
    %{
      "meta" => %{"event_name" => "subscription_expired"},
      "data" => %{
        "id" => subscription_id,
        "attributes" => %{}
      }
    }
  end

  defp subscription_payment_success_payload(subscription_id) do
    %{
      "meta" => %{"event_name" => "subscription_payment_success"},
      "data" => %{
        "id" => "payment_123",
        "attributes" => %{
          "subscription_id" => subscription_id
        }
      }
    }
  end

  defp subscription_payment_failed_payload(subscription_id) do
    %{
      "meta" => %{"event_name" => "subscription_payment_failed"},
      "data" => %{
        "id" => "payment_123",
        "attributes" => %{
          "subscription_id" => subscription_id
        }
      }
    }
  end
end
