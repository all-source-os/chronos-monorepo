defmodule QueryServiceExWeb.WebhookControllerTest do
  @moduledoc """
  Tests for WebhookController LemonSqueezy webhooks.

  The webhook controller now only verifies signatures, logs events, and acknowledges.
  Subscription management is owned by the Control Plane (Go service).
  No database required.
  """
  use QueryServiceExWeb.ConnCase

  @webhook_secret "test_webhook_secret_for_testing"

  setup %{conn: conn} do
    # Configure webhook secret for signature verification
    Application.put_env(:query_service_ex, :lemon_squeezy, webhook_secret: @webhook_secret)

    on_exit(fn ->
      Application.put_env(:query_service_ex, :lemon_squeezy, webhook_secret: nil)
    end)

    conn =
      conn
      |> put_req_header("content-type", "application/json")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn}
  end

  describe "POST /api/webhooks/lemonsqueezy" do
    test "returns unauthorized for invalid signature", %{conn: conn} do
      payload = subscription_created_payload("tenant-123")

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

    test "acknowledges subscription_created event with valid signature", %{conn: conn} do
      payload = subscription_created_payload("tenant-123")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_updated event with valid signature", %{conn: conn} do
      payload = subscription_updated_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_cancelled event", %{conn: conn} do
      payload = subscription_cancelled_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_resumed event", %{conn: conn} do
      payload = subscription_resumed_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_expired event", %{conn: conn} do
      payload = subscription_expired_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_payment_success event", %{conn: conn} do
      payload = subscription_payment_success_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
    end

    test "acknowledges subscription_payment_failed event", %{conn: conn} do
      payload = subscription_payment_failed_payload("sub_67890")
      body = Jason.encode!(payload)
      signature = compute_signature(body)

      conn =
        conn
        |> put_raw_body(body)
        |> put_req_header("x-signature", signature)
        |> post("/api/webhooks/lemonsqueezy", payload)

      assert json_response(conn, 200)["received"] == true
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
          "customer_id" => 12_345,
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
