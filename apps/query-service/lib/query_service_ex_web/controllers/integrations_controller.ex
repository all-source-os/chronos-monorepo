defmodule QueryServiceExWeb.IntegrationsController do
  @moduledoc """
  Controller for message queue integration management.

  Provides endpoints for:
  - Kafka producer/consumer status and configuration
  - RabbitMQ producer/consumer status and configuration
  - Publishing events to message queues
  """

  use QueryServiceExWeb, :controller
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Integrations.Kafka
  alias QueryServiceEx.Integrations.RabbitMQ
  alias QueryServiceExWeb.Schemas.Common
  alias QueryServiceExWeb.Schemas.Integrations

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Integrations"])

  operation(:kafka_status,
    summary: "Get Kafka integration status",
    description: "Returns the current status of Kafka producer and consumer",
    responses: [
      ok: {"Kafka status", "application/json", Integrations.KafkaStatusResponse}
    ]
  )

  @doc """
  Get Kafka integration status.
  """
  def kafka_status(conn, _params) do
    producer_status = Kafka.Producer.status()
    consumer_status = Kafka.Consumer.status()

    json(conn, %{
      producer: producer_status,
      consumer: consumer_status,
      enabled: Kafka.Producer.enabled?() || Kafka.Consumer.enabled?()
    })
  end

  operation(:kafka_config,
    summary: "Get Kafka configuration",
    description: "Returns the current Kafka producer configuration",
    responses: [
      ok: {"Kafka config", "application/json", Integrations.KafkaConfigResponse}
    ]
  )

  @doc """
  Get Kafka configuration.
  """
  def kafka_config(conn, _params) do
    config = Kafka.Producer.config()
    json(conn, config)
  end

  operation(:kafka_publish,
    summary: "Publish event to Kafka",
    description: "Publishes an event to the configured Kafka topic",
    request_body: {"Event payload", "application/json", Integrations.PublishEventRequest},
    responses: [
      ok: {"Event published", "application/json", Integrations.PublishResponse},
      unprocessable_entity: {"Kafka disabled or error", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Publish an event to Kafka.
  """
  def kafka_publish(conn, %{"event" => event} = params) do
    opts = build_publish_opts(params)

    case Kafka.Producer.publish(event, opts) do
      :ok ->
        json(conn, %{status: "published", destination: "kafka"})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: format_error(reason)})
    end
  end

  operation(:rabbitmq_status,
    summary: "Get RabbitMQ integration status",
    description: "Returns the current status of RabbitMQ producer and consumer",
    responses: [
      ok: {"RabbitMQ status", "application/json", Integrations.RabbitMQStatusResponse}
    ]
  )

  @doc """
  Get RabbitMQ integration status.
  """
  def rabbitmq_status(conn, _params) do
    producer_status = RabbitMQ.Producer.status()
    consumer_status = RabbitMQ.Consumer.status()

    json(conn, %{
      producer: producer_status,
      consumer: consumer_status,
      enabled: RabbitMQ.Producer.enabled?() || RabbitMQ.Consumer.enabled?()
    })
  end

  operation(:rabbitmq_config,
    summary: "Get RabbitMQ configuration",
    description: "Returns the current RabbitMQ producer configuration",
    responses: [
      ok: {"RabbitMQ config", "application/json", Integrations.RabbitMQConfigResponse}
    ]
  )

  @doc """
  Get RabbitMQ configuration.
  """
  def rabbitmq_config(conn, _params) do
    config = RabbitMQ.Producer.config()
    json(conn, config)
  end

  operation(:rabbitmq_publish,
    summary: "Publish event to RabbitMQ",
    description: "Publishes an event to the configured RabbitMQ exchange",
    request_body: {"Event payload", "application/json", Integrations.PublishEventRequest},
    responses: [
      ok: {"Event published", "application/json", Integrations.PublishResponse},
      unprocessable_entity: {"RabbitMQ disabled or error", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Publish an event to RabbitMQ.
  """
  def rabbitmq_publish(conn, %{"event" => event} = params) do
    opts = build_publish_opts(params)

    case RabbitMQ.Producer.publish(event, opts) do
      :ok ->
        json(conn, %{status: "published", destination: "rabbitmq"})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: format_error(reason)})
    end
  end

  operation(:all_status,
    summary: "Get all integrations status",
    description: "Returns the status of all message queue integrations",
    responses: [
      ok:
        {"All integrations status", "application/json",
         Integrations.AllIntegrationsStatusResponse}
    ]
  )

  @doc """
  Get status of all integrations.
  """
  def all_status(conn, _params) do
    kafka_producer = Kafka.Producer.status()
    kafka_consumer = Kafka.Consumer.status()
    rabbitmq_producer = RabbitMQ.Producer.status()
    rabbitmq_consumer = RabbitMQ.Consumer.status()

    json(conn, %{
      kafka: %{
        producer: kafka_producer,
        consumer: kafka_consumer,
        enabled: Kafka.Producer.enabled?() || Kafka.Consumer.enabled?()
      },
      rabbitmq: %{
        producer: rabbitmq_producer,
        consumer: rabbitmq_consumer,
        enabled: RabbitMQ.Producer.enabled?() || RabbitMQ.Consumer.enabled?()
      }
    })
  end

  ## Private Functions

  defp build_publish_opts(params) do
    opts = []

    opts =
      if params["topic"] do
        Keyword.put(opts, :topic, params["topic"])
      else
        opts
      end

    opts =
      if params["routing_key"] do
        Keyword.put(opts, :routing_key, params["routing_key"])
      else
        opts
      end

    opts =
      if params["exchange"] do
        Keyword.put(opts, :exchange, params["exchange"])
      else
        opts
      end

    if params["partition_key"] do
      Keyword.put(opts, :partition_key, params["partition_key"])
    else
      opts
    end
  end

  defp format_error(:kafka_disabled), do: "Kafka integration is disabled"
  defp format_error(:rabbitmq_disabled), do: "RabbitMQ integration is disabled"
  defp format_error({:not_connected, status}), do: "Not connected: #{status}"
  defp format_error(reason), do: inspect(reason)
end
