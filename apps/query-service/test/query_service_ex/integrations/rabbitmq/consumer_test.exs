defmodule QueryServiceEx.Integrations.RabbitMQ.ConsumerTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Integrations.RabbitMQ.Consumer

  describe "enabled?/0" do
    test "returns false when not configured" do
      refute Consumer.enabled?()
    end

    test "returns true when RABBITMQ_CONSUMER_ENABLED=true" do
      System.put_env("RABBITMQ_CONSUMER_ENABLED", "true")
      on_exit(fn -> System.delete_env("RABBITMQ_CONSUMER_ENABLED") end)

      assert Consumer.enabled?()
    end

    test "returns false when RABBITMQ_CONSUMER_ENABLED is not 'true'" do
      System.put_env("RABBITMQ_CONSUMER_ENABLED", "false")
      on_exit(fn -> System.delete_env("RABBITMQ_CONSUMER_ENABLED") end)

      refute Consumer.enabled?()
    end
  end

  describe "status/0" do
    test "returns not_running when consumer is not started" do
      status = Consumer.status()
      assert status.status == :not_running
      assert Map.has_key?(status, :enabled)
    end
  end

  describe "start_link/1 when disabled" do
    test "returns :ignore when consumer is disabled" do
      assert :ignore = Consumer.start_link()
    end
  end
end
