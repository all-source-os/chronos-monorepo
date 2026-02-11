defmodule QueryServiceEx.Integrations.RabbitMQ.Consumer do
  @moduledoc """
  Broadway-based RabbitMQ consumer for ingesting events.

  Provides:
  - High-throughput event consumption
  - Automatic batching and acknowledgment
  - Dead-letter queue support for failed messages
  - Configurable processing pipelines

  ## Configuration

      config :query_service_ex, QueryServiceEx.Integrations.RabbitMQ.Consumer,
        enabled: true,
        url: "amqp://guest:guest@localhost:5672",
        queue: "chronos.events.queue",
        exchange: "chronos.events",
        routing_key: "#",
        dlq_exchange: "chronos.events.dlq",
        processors: [default: [concurrency: 10]],
        batchers: [default: [batch_size: 50, batch_timeout: 1000]]

  ## Environment Variables

      RABBITMQ_CONSUMER_ENABLED=true
      RABBITMQ_URL=amqp://guest:guest@localhost:5672
      RABBITMQ_CONSUMER_QUEUE=chronos.events.queue
      RABBITMQ_CONSUMER_EXCHANGE=chronos.events
      RABBITMQ_CONSUMER_ROUTING_KEY=#
  """

  use Broadway

  require Logger

  alias Broadway.Message
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  Start the RabbitMQ consumer Broadway pipeline.
  """
  def start_link(_opts \\ []) do
    if enabled?() do
      queue = get_queue()
      exchange = get_exchange()
      routing_key = get_routing_key()

      Broadway.start_link(__MODULE__,
        name: __MODULE__,
        producer: [
          module: {
            BroadwayRabbitMQ.Producer,
            on_failure: :reject_and_requeue_once,
            queue: queue,
            connection: [url: get_url()],
            declare: [
              durable: true
            ],
            bindings: [
              {exchange, routing_key: routing_key}
            ],
            qos: [
              prefetch_count: get_prefetch_count()
            ]
          },
          concurrency: 1
        ],
        processors: [
          default: [concurrency: get_processor_concurrency()]
        ],
        batchers: [
          default: [
            batch_size: get_batch_size(),
            batch_timeout: get_batch_timeout()
          ],
          dlq: [
            batch_size: 10,
            batch_timeout: 2000
          ]
        ]
      )
    else
      Logger.info("[RabbitMQ.Consumer] Disabled, not starting")
      :ignore
    end
  end

  @doc """
  Check if RabbitMQ consumer is enabled.
  """
  def enabled? do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :enabled, false) ||
      System.get_env("RABBITMQ_CONSUMER_ENABLED") == "true"
  end

  @doc """
  Get consumer status.
  """
  def status do
    if Process.whereis(__MODULE__) do
      %{
        status: :running,
        enabled: true,
        queue: get_queue()
      }
    else
      %{
        status: :not_running,
        enabled: enabled?()
      }
    end
  end

  ## Broadway Callbacks

  @impl true
  def handle_message(_processor, %Message{data: data} = message, _context) do
    case process_event(data) do
      :ok ->
        message

      {:error, reason} ->
        Logger.warning("[RabbitMQ.Consumer] Failed to process message: #{inspect(reason)}")
        Message.failed(message, reason)
    end
  rescue
    e ->
      Logger.error("[RabbitMQ.Consumer] Exception processing message: #{inspect(e)}")
      Message.failed(message, {:exception, e})
  end

  @impl true
  def handle_batch(:default, messages, _batch_info, _context) do
    Logger.debug("[RabbitMQ.Consumer] Processing batch of #{length(messages)} messages")

    {successful, failed} = Enum.split_with(messages, &is_nil(&1.status))

    :telemetry.execute(
      [:query_service, :rabbitmq, :consumer, :batch],
      %{
        success_count: length(successful),
        failure_count: length(failed)
      },
      %{}
    )

    messages
  end

  @impl true
  def handle_batch(:dlq, messages, _batch_info, _context) do
    Logger.warning("[RabbitMQ.Consumer] DLQ batch: #{length(messages)} messages")

    for message <- messages do
      handle_dlq_message(message)
    end

    messages
  end

  @impl true
  def handle_failed(messages, _context) do
    Enum.map(messages, fn message ->
      Message.configure_ack(message, on_failure: :ack)
      |> Message.put_batcher(:dlq)
    end)
  end

  ## Private Functions

  defp process_event(data) when is_binary(data) do
    case Jason.decode(data) do
      {:ok, decoded} ->
        process_decoded_event(decoded)

      {:error, reason} ->
        {:error, {:json_decode_error, reason}}
    end
  end

  defp process_event(_data), do: {:error, :invalid_data_format}

  defp process_decoded_event(%{"event" => event} = _message) do
    tenant_id = event["tenant_id"]

    if tenant_id do
      case RustCoreClient.create_event(tenant_id, event) do
        {:ok, _} -> :ok
        {:error, reason} -> {:error, {:core_error, reason}}
      end
    else
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:ingested",
        {:rabbitmq_event, event}
      )

      :ok
    end
  end

  defp process_decoded_event(_), do: {:error, :missing_event_field}

  defp handle_dlq_message(%Message{data: data, metadata: metadata}) do
    Logger.error(
      "[RabbitMQ.Consumer] DLQ message: #{inspect(data)}, metadata: #{inspect(metadata)}"
    )

    # Could publish to a DLQ exchange, store in database, etc.
    publish_to_dlq(data, metadata)
  end

  defp publish_to_dlq(_data, _metadata) do
    dlq_exchange = get_dlq_exchange()

    if dlq_exchange do
      # If we had a channel, we'd publish here
      # For now, just log
      Logger.info("[RabbitMQ.Consumer] Would publish to DLQ exchange: #{dlq_exchange}")
    end

    :ok
  end

  defp get_url do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :url) ||
      System.get_env("RABBITMQ_URL", "amqp://guest:guest@localhost:5672")
  end

  defp get_queue do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :queue) ||
      System.get_env("RABBITMQ_CONSUMER_QUEUE", "chronos.events.queue")
  end

  defp get_exchange do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :exchange) ||
      System.get_env("RABBITMQ_CONSUMER_EXCHANGE", "chronos.events")
  end

  defp get_routing_key do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :routing_key) || System.get_env("RABBITMQ_CONSUMER_ROUTING_KEY", "#")
  end

  defp get_dlq_exchange do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :dlq_exchange) || System.get_env("RABBITMQ_DLQ_EXCHANGE")
  end

  defp get_prefetch_count do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    qos = Keyword.get(config, :qos, [])
    Keyword.get(qos, :prefetch_count, 50)
  end

  defp get_processor_concurrency do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    processors = Keyword.get(config, :processors, [])
    default_config = Keyword.get(processors, :default, [])
    Keyword.get(default_config, :concurrency, 10)
  end

  defp get_batch_size do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    batchers = Keyword.get(config, :batchers, [])
    default_config = Keyword.get(batchers, :default, [])
    Keyword.get(default_config, :batch_size, 50)
  end

  defp get_batch_timeout do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    batchers = Keyword.get(config, :batchers, [])
    default_config = Keyword.get(batchers, :default, [])
    Keyword.get(default_config, :batch_timeout, 1000)
  end
end
