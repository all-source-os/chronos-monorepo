defmodule QueryServiceEx.Integrations.RabbitMQ.ProducerTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Integrations.RabbitMQ.Producer

  describe "enabled?/0" do
    test "returns false when not configured" do
      refute Producer.enabled?()
    end

    test "returns true when RABBITMQ_ENABLED=true" do
      System.put_env("RABBITMQ_ENABLED", "true")
      on_exit(fn -> System.delete_env("RABBITMQ_ENABLED") end)

      assert Producer.enabled?()
    end

    test "returns false when RABBITMQ_ENABLED is not 'true'" do
      System.put_env("RABBITMQ_ENABLED", "false")
      on_exit(fn -> System.delete_env("RABBITMQ_ENABLED") end)

      refute Producer.enabled?()
    end
  end

  describe "status/0 when not started" do
    test "returns not_started status" do
      status = Producer.status()
      assert status == %{status: :not_started}
    end
  end

  describe "config/0 when not started" do
    test "returns not_started config" do
      config = Producer.config()
      assert config == %{enabled: false, status: :not_started}
    end
  end

  describe "publish/2 when disabled" do
    test "returns error when rabbitmq is disabled" do
      event = %{entity_id: "test-123", event_type: "test.event"}
      assert {:error, :rabbitmq_disabled} = Producer.publish(event)
    end
  end

  describe "publish_batch/2 when disabled" do
    test "returns error when rabbitmq is disabled" do
      events = [
        %{entity_id: "test-1", event_type: "test.event"},
        %{entity_id: "test-2", event_type: "test.event"}
      ]

      assert {:error, :rabbitmq_disabled} = Producer.publish_batch(events)
    end
  end
end
