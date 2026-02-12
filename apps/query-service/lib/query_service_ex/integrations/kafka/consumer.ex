defmodule QueryServiceEx.Integrations.Kafka.Consumer do
  @moduledoc """
  Broadway-based Kafka consumer for ingesting events.

  Provides:
  - High-throughput event consumption
  - Automatic batching and acknowledgment
  - Dead-letter queue support for failed messages
  - Configurable processing pipelines

  ## Configuration

      config :query_service_ex, QueryServiceEx.Integrations.Kafka.Consumer,
        enabled: true,
        brokers: [{"localhost", 9092}],
        group_id: "query-service-consumer",
        topics: ["allsource-events"],
        processors: [default: [concurrency: 10]],
        batchers: [default: [batch_size: 100, batch_timeout: 1000]]

  ## Environment Variables

      KAFKA_CONSUMER_ENABLED=true
      KAFKA_BROKERS=localhost:9092
      KAFKA_CONSUMER_GROUP_ID=query-service-consumer
      KAFKA_CONSUMER_TOPICS=allsource-events
  """

  use Broadway

  require Logger

  alias Broadway.Message
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  Start the Kafka consumer Broadway pipeline.
  """
  def start_link(_opts \\ []) do
    if enabled?() do
      Broadway.start_link(__MODULE__,
        name: __MODULE__,
        producer: [
          module: {
            BroadwayKafka.Producer,
            [
              hosts: get_brokers(),
              group_id: get_group_id(),
              topics: get_topics(),
              offset_commit_on_ack: true,
              offset_reset_policy: :earliest
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
      Logger.info("[Kafka.Consumer] Disabled, not starting")
      :ignore
    end
  end

  @doc """
  Check if Kafka consumer is enabled.
  """
  def enabled? do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :enabled, false) ||
      System.get_env("KAFKA_CONSUMER_ENABLED") == "true"
  end

  @doc """
  Get consumer status.
  """
  def status do
    if Process.whereis(__MODULE__) do
      %{
        status: :running,
        enabled: true
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
        Logger.warning("[Kafka.Consumer] Failed to process message: #{inspect(reason)}")
        Message.failed(message, reason)
    end
  rescue
    e ->
      Logger.error("[Kafka.Consumer] Exception processing message: #{inspect(e)}")
      Message.failed(message, {:exception, e})
  end

  @impl true
  def handle_batch(:default, messages, _batch_info, _context) do
    # Log batch processing
    Logger.debug("[Kafka.Consumer] Processing batch of #{length(messages)} messages")

    # Separate successful and failed messages
    {successful, failed} = Enum.split_with(messages, &is_nil(&1.status))

    # Emit telemetry
    :telemetry.execute(
      [:query_service, :kafka, :consumer, :batch],
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
    # Handle dead-letter queue messages
    Logger.warning("[Kafka.Consumer] DLQ batch: #{length(messages)} messages")

    for message <- messages do
      handle_dlq_message(message)
    end

    messages
  end

  @impl true
  def handle_failed(messages, _context) do
    # Route failed messages to DLQ batcher
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
    # Extract tenant_id if present
    tenant_id = event["tenant_id"]

    # Forward event to Core for storage
    if tenant_id do
      case RustCoreClient.create_event(tenant_id, event) do
        {:ok, _} -> :ok
        {:error, reason} -> {:error, {:core_error, reason}}
      end
    else
      # Broadcast via PubSub for events without tenant_id
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:ingested",
        {:kafka_event, event}
      )

      :ok
    end
  end

  defp process_decoded_event(_), do: {:error, :missing_event_field}

  defp handle_dlq_message(%Message{data: data, metadata: metadata}) do
    # Log to DLQ storage or external system
    Logger.error("[Kafka.Consumer] DLQ message: #{inspect(data)}, metadata: #{inspect(metadata)}")

    # Could store in database, send to another topic, etc.
    # For now, just log and acknowledge
    :ok
  end

  defp get_brokers do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    case Keyword.get(config, :brokers) do
      nil ->
        brokers_str = System.get_env("KAFKA_BROKERS", "localhost:9092")

        brokers_str
        |> String.split(",")
        |> Enum.map(fn broker ->
          [host, port] = String.split(String.trim(broker), ":")
          {String.to_charlist(host), String.to_integer(port)}
        end)

      brokers when is_list(brokers) ->
        Enum.map(brokers, fn {host, port} ->
          {String.to_charlist(to_string(host)), port}
        end)
    end
  end

  defp get_group_id do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :group_id) ||
      System.get_env("KAFKA_CONSUMER_GROUP_ID", "query-service-consumer")
  end

  defp get_topics do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    case Keyword.get(config, :topics) do
      nil ->
        topics_str = System.get_env("KAFKA_CONSUMER_TOPICS", "allsource-events")
        String.split(topics_str, ",") |> Enum.map(&String.trim/1)

      topics when is_list(topics) ->
        topics
    end
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
    Keyword.get(default_config, :batch_size, 100)
  end

  defp get_batch_timeout do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    batchers = Keyword.get(config, :batchers, [])
    default_config = Keyword.get(batchers, :default, [])
    Keyword.get(default_config, :batch_timeout, 1000)
  end
end
