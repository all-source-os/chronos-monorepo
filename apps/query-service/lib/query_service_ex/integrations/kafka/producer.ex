defmodule QueryServiceEx.Integrations.Kafka.Producer do
  @moduledoc """
  Kafka producer for publishing events to Kafka topics.

  Provides:
  - Configurable broker connections
  - Topic-based event routing
  - Partition key support for ordering
  - Automatic serialization to JSON

  ## Configuration

      config :query_service_ex, QueryServiceEx.Integrations.Kafka.Producer,
        enabled: true,
        brokers: [{"localhost", 9092}],
        client_id: :query_service_kafka,
        default_topic: "allsource-events",
        producer_config: [
          required_acks: :all,
          max_retries: 3
        ]

  ## Environment Variables

      KAFKA_ENABLED=true
      KAFKA_BROKERS=localhost:9092,broker2:9092
      KAFKA_DEFAULT_TOPIC=allsource-events
      KAFKA_CLIENT_ID=query-service
  """

  use GenServer

  require Logger

  @default_topic "allsource-events"

  ## Client API

  @doc """
  Start the Kafka producer.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Publish an event to Kafka.

  ## Parameters
    * `event` - Event map with at least `entity_id` and `event_type`
    * `opts` - Options:
      * `:topic` - Override default topic
      * `:partition_key` - Key for partitioning (default: entity_id)
      * `:headers` - Additional Kafka headers

  ## Returns
    * `:ok` - Event published successfully
    * `{:error, reason}` - Failed to publish

  ## Examples

      Kafka.Producer.publish(%{
        entity_id: "user-123",
        event_type: "user.created",
        payload: %{email: "user@example.com"}
      })
  """
  def publish(event, opts \\ []) do
    if enabled?() do
      GenServer.call(__MODULE__, {:publish, event, opts})
    else
      {:error, :kafka_disabled}
    end
  end

  @doc """
  Publish multiple events to Kafka.

  ## Parameters
    * `events` - List of event maps
    * `opts` - Same options as `publish/2`

  ## Returns
    * `{:ok, count}` - Number of events published
    * `{:error, reason}` - Failed to publish
  """
  def publish_batch(events, opts \\ []) when is_list(events) do
    if enabled?() do
      GenServer.call(__MODULE__, {:publish_batch, events, opts})
    else
      {:error, :kafka_disabled}
    end
  end

  @doc """
  Check if Kafka integration is enabled.
  """
  def enabled? do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :enabled, false) || System.get_env("KAFKA_ENABLED") == "true"
  end

  @doc """
  Get current configuration.
  """
  def config do
    GenServer.call(__MODULE__, :config)
  catch
    :exit, _ -> %{enabled: false, status: :not_started}
  end

  @doc """
  Get producer status.
  """
  def status do
    GenServer.call(__MODULE__, :status)
  catch
    :exit, _ -> %{status: :not_started}
  end

  ## Server Callbacks

  @impl true
  def init(_opts) do
    if enabled?() do
      brokers = get_brokers()
      client_id = get_client_id()
      default_topic = get_default_topic()

      Logger.info("[Kafka.Producer] Starting with brokers: #{inspect(brokers)}")

      # Start brod client
      case :brod.start_client(brokers, client_id, _client_config = []) do
        :ok ->
          # Start producer for default topic
          case :brod.start_producer(client_id, default_topic, _producer_config = []) do
            :ok ->
              Logger.info("[Kafka.Producer] Started successfully")

              {:ok,
               %{
                 client_id: client_id,
                 brokers: brokers,
                 default_topic: default_topic,
                 status: :connected,
                 published_count: 0,
                 error_count: 0
               }}

            {:error, reason} ->
              Logger.warning("[Kafka.Producer] Failed to start producer: #{inspect(reason)}")
              {:ok, %{status: :producer_error, error: reason}}
          end

        {:error, reason} ->
          Logger.warning("[Kafka.Producer] Failed to start client: #{inspect(reason)}")
          {:ok, %{status: :client_error, error: reason}}
      end
    else
      Logger.info("[Kafka.Producer] Disabled, not starting")
      {:ok, %{status: :disabled}}
    end
  end

  @impl true
  def handle_call({:publish, event, opts}, _from, %{status: :connected} = state) do
    topic = Keyword.get(opts, :topic, state.default_topic)
    partition_key = Keyword.get(opts, :partition_key, event[:entity_id] || "default")
    headers = Keyword.get(opts, :headers, [])

    message = encode_event(event, headers)

    case :brod.produce_sync(state.client_id, topic, :hash, partition_key, message) do
      :ok ->
        {:reply, :ok, %{state | published_count: state.published_count + 1}}

      {:error, reason} = error ->
        Logger.error("[Kafka.Producer] Publish failed: #{inspect(reason)}")
        {:reply, error, %{state | error_count: state.error_count + 1}}
    end
  end

  def handle_call({:publish, _event, _opts}, _from, state) do
    {:reply, {:error, {:not_connected, state.status}}, state}
  end

  @impl true
  def handle_call({:publish_batch, events, opts}, _from, %{status: :connected} = state) do
    topic = Keyword.get(opts, :topic, state.default_topic)
    headers = Keyword.get(opts, :headers, [])

    messages =
      Enum.map(events, fn event ->
        partition_key = event[:entity_id] || "default"
        {partition_key, encode_event(event, headers)}
      end)

    # Group by partition key for efficient batching
    results =
      Enum.map(messages, fn {key, value} ->
        :brod.produce_sync(state.client_id, topic, :hash, key, value)
      end)

    success_count = Enum.count(results, &(&1 == :ok))
    error_count = length(results) - success_count

    {:reply, {:ok, success_count},
     %{
       state
       | published_count: state.published_count + success_count,
         error_count: state.error_count + error_count
     }}
  end

  def handle_call({:publish_batch, _events, _opts}, _from, state) do
    {:reply, {:error, {:not_connected, state.status}}, state}
  end

  @impl true
  def handle_call(:config, _from, state) do
    config = %{
      enabled: enabled?(),
      brokers: Map.get(state, :brokers, []),
      default_topic: Map.get(state, :default_topic, @default_topic),
      client_id: Map.get(state, :client_id)
    }

    {:reply, config, state}
  end

  @impl true
  def handle_call(:status, _from, state) do
    status = %{
      status: state.status,
      published_count: Map.get(state, :published_count, 0),
      error_count: Map.get(state, :error_count, 0)
    }

    {:reply, status, state}
  end

  ## Private Functions

  defp get_brokers do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    case Keyword.get(config, :brokers) do
      nil ->
        # Parse from environment
        brokers_str = System.get_env("KAFKA_BROKERS", "localhost:9092")

        brokers_str
        |> String.split(",")
        |> Enum.map(fn broker ->
          [host, port] = String.split(String.trim(broker), ":")
          {host, String.to_integer(port)}
        end)

      brokers when is_list(brokers) ->
        brokers
    end
  end

  defp get_client_id do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :client_id) ||
      System.get_env("KAFKA_CLIENT_ID", "query_service") |> String.to_atom()
  end

  defp get_default_topic do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :default_topic) || System.get_env("KAFKA_DEFAULT_TOPIC", @default_topic)
  end

  defp encode_event(event, headers) do
    timestamp = DateTime.utc_now() |> DateTime.to_iso8601()

    message = %{
      event: event,
      metadata: %{
        published_at: timestamp,
        source: "query-service"
      }
    }

    # Add headers as metadata if provided
    message =
      if Enum.empty?(headers) do
        message
      else
        put_in(message, [:metadata, :headers], Map.new(headers))
      end

    Jason.encode!(message)
  end
end
