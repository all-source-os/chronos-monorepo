defmodule QueryServiceExWeb.Schemas.Integrations do
  @moduledoc """
  OpenAPI schemas for message queue integrations.
  """

  alias OpenApiSpex.Schema

  defmodule KafkaStatusResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "KafkaStatusResponse",
      description: "Kafka integration status",
      type: :object,
      properties: %{
        producer: %Schema{
          type: :object,
          properties: %{
            status: %Schema{type: :string, description: "Producer status"},
            published_count: %Schema{type: :integer, description: "Number of published messages"},
            error_count: %Schema{type: :integer, description: "Number of errors"}
          }
        },
        consumer: %Schema{
          type: :object,
          properties: %{
            status: %Schema{type: :string, description: "Consumer status"},
            enabled: %Schema{type: :boolean, description: "Whether consumer is enabled"}
          }
        },
        enabled: %Schema{type: :boolean, description: "Whether Kafka is enabled"}
      },
      example: %{
        producer: %{status: "connected", published_count: 100, error_count: 0},
        consumer: %{status: "running", enabled: true},
        enabled: true
      }
    })
  end

  defmodule KafkaConfigResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "KafkaConfigResponse",
      description: "Kafka configuration",
      type: :object,
      properties: %{
        enabled: %Schema{type: :boolean, description: "Whether Kafka is enabled"},
        brokers: %Schema{
          type: :array,
          items: %Schema{type: :array, items: %Schema{type: :string}},
          description: "List of broker host:port pairs"
        },
        default_topic: %Schema{type: :string, description: "Default topic for publishing"},
        client_id: %Schema{type: :string, description: "Kafka client ID"}
      },
      example: %{
        enabled: true,
        brokers: [["localhost", 9092]],
        default_topic: "chronos-events",
        client_id: "query_service"
      }
    })
  end

  defmodule RabbitMQStatusResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "RabbitMQStatusResponse",
      description: "RabbitMQ integration status",
      type: :object,
      properties: %{
        producer: %Schema{
          type: :object,
          properties: %{
            status: %Schema{type: :string, description: "Producer status"},
            published_count: %Schema{type: :integer, description: "Number of published messages"},
            error_count: %Schema{type: :integer, description: "Number of errors"}
          }
        },
        consumer: %Schema{
          type: :object,
          properties: %{
            status: %Schema{type: :string, description: "Consumer status"},
            enabled: %Schema{type: :boolean, description: "Whether consumer is enabled"},
            queue: %Schema{type: :string, description: "Queue name"}
          }
        },
        enabled: %Schema{type: :boolean, description: "Whether RabbitMQ is enabled"}
      },
      example: %{
        producer: %{status: "connected", published_count: 50, error_count: 0},
        consumer: %{status: "running", enabled: true, queue: "chronos.events.queue"},
        enabled: true
      }
    })
  end

  defmodule RabbitMQConfigResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "RabbitMQConfigResponse",
      description: "RabbitMQ configuration",
      type: :object,
      properties: %{
        enabled: %Schema{type: :boolean, description: "Whether RabbitMQ is enabled"},
        exchange: %Schema{type: :string, description: "Exchange name"},
        status: %Schema{type: :string, description: "Connection status"}
      },
      example: %{
        enabled: true,
        exchange: "chronos.events",
        status: "connected"
      }
    })
  end

  defmodule PublishEventRequest do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "PublishEventRequest",
      description: "Request to publish an event",
      type: :object,
      required: [:event],
      properties: %{
        event: %Schema{
          type: :object,
          description: "Event to publish",
          properties: %{
            entity_id: %Schema{type: :string, description: "Entity ID"},
            event_type: %Schema{type: :string, description: "Event type"},
            payload: %Schema{type: :object, description: "Event payload"}
          },
          required: [:entity_id, :event_type]
        },
        topic: %Schema{type: :string, description: "Override topic (Kafka)"},
        exchange: %Schema{type: :string, description: "Override exchange (RabbitMQ)"},
        routing_key: %Schema{type: :string, description: "Routing key (RabbitMQ)"},
        partition_key: %Schema{type: :string, description: "Partition key (Kafka)"}
      },
      example: %{
        event: %{
          entity_id: "user-123",
          event_type: "user.created",
          payload: %{email: "user@example.com"}
        }
      }
    })
  end

  defmodule PublishResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "PublishResponse",
      description: "Response after publishing an event",
      type: :object,
      properties: %{
        status: %Schema{type: :string, description: "Publish status"},
        destination: %Schema{type: :string, description: "Destination (kafka or rabbitmq)"}
      },
      example: %{
        status: "published",
        destination: "kafka"
      }
    })
  end

  defmodule AllIntegrationsStatusResponse do
    @moduledoc false
    require OpenApiSpex

    OpenApiSpex.schema(%{
      title: "AllIntegrationsStatusResponse",
      description: "Status of all message queue integrations",
      type: :object,
      properties: %{
        kafka: %Schema{
          type: :object,
          properties: %{
            producer: %Schema{type: :object},
            consumer: %Schema{type: :object},
            enabled: %Schema{type: :boolean}
          }
        },
        rabbitmq: %Schema{
          type: :object,
          properties: %{
            producer: %Schema{type: :object},
            consumer: %Schema{type: :object},
            enabled: %Schema{type: :boolean}
          }
        }
      },
      example: %{
        kafka: %{
          producer: %{status: "connected", published_count: 100, error_count: 0},
          consumer: %{status: "running", enabled: true},
          enabled: true
        },
        rabbitmq: %{
          producer: %{status: "connected", published_count: 50, error_count: 0},
          consumer: %{status: "running", enabled: true},
          enabled: true
        }
      }
    })
  end
end
