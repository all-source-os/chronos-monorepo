defmodule QueryServiceEx.Integrations.RabbitMQ.Producer do
  @moduledoc """
  RabbitMQ producer for publishing events to exchanges/queues.

  Provides:
  - Configurable broker connections
  - Exchange-based routing
  - Routing key support
  - Automatic serialization to JSON
  - Connection pooling and recovery

  ## Configuration

      config :query_service_ex, QueryServiceEx.Integrations.RabbitMQ.Producer,
        enabled: true,
        url: "amqp://guest:guest@localhost:5672",
        exchange: "allsource.events",
        exchange_type: :topic,
        durable: true

  ## Environment Variables

      RABBITMQ_ENABLED=true
      RABBITMQ_URL=amqp://guest:guest@localhost:5672
      RABBITMQ_EXCHANGE=allsource.events
      RABBITMQ_EXCHANGE_TYPE=topic
  """

  use GenServer

  require Logger

  @default_exchange "allsource.events"

  ## Client API

  @doc """
  Start the RabbitMQ producer.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Publish an event to RabbitMQ.

  ## Parameters
    * `event` - Event map with at least `entity_id` and `event_type`
    * `opts` - Options:
      * `:routing_key` - Override routing key (default: event_type)
      * `:exchange` - Override exchange
      * `:headers` - Additional AMQP headers

  ## Returns
    * `:ok` - Event published successfully
    * `{:error, reason}` - Failed to publish

  ## Examples

      RabbitMQ.Producer.publish(%{
        entity_id: "user-123",
        event_type: "user.created",
        payload: %{email: "user@example.com"}
      })
  """
  def publish(event, opts \\ []) do
    if enabled?() do
      GenServer.call(__MODULE__, {:publish, event, opts})
    else
      {:error, :rabbitmq_disabled}
    end
  end

  @doc """
  Publish multiple events to RabbitMQ.

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
      {:error, :rabbitmq_disabled}
    end
  end

  @doc """
  Check if RabbitMQ integration is enabled.
  """
  def enabled? do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :enabled, false) || System.get_env("RABBITMQ_ENABLED") == "true"
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
      url = get_url()
      exchange = get_exchange()
      exchange_type = get_exchange_type()
      durable = get_durable()

      Logger.info("[RabbitMQ.Producer] Connecting to #{sanitize_url(url)}")

      case AMQP.Connection.open(url) do
        {:ok, conn} ->
          # Monitor connection for failures
          Process.monitor(conn.pid)

          case AMQP.Channel.open(conn) do
            {:ok, channel} ->
              # Declare exchange
              :ok =
                AMQP.Exchange.declare(channel, exchange, exchange_type,
                  durable: durable,
                  auto_delete: false
                )

              Logger.info("[RabbitMQ.Producer] Connected and exchange declared")

              {:ok,
               %{
                 connection: conn,
                 channel: channel,
                 exchange: exchange,
                 status: :connected,
                 published_count: 0,
                 error_count: 0
               }}

            {:error, reason} ->
              Logger.warning("[RabbitMQ.Producer] Failed to open channel: #{inspect(reason)}")
              AMQP.Connection.close(conn)
              {:ok, %{status: :channel_error, error: reason}}
          end

        {:error, reason} ->
          Logger.warning("[RabbitMQ.Producer] Failed to connect: #{inspect(reason)}")
          {:ok, %{status: :connection_error, error: reason}}
      end
    else
      Logger.info("[RabbitMQ.Producer] Disabled, not starting")
      {:ok, %{status: :disabled}}
    end
  end

  @impl true
  def handle_call({:publish, event, opts}, _from, %{status: :connected} = state) do
    exchange = Keyword.get(opts, :exchange, state.exchange)
    routing_key = Keyword.get(opts, :routing_key, event[:event_type] || "event")
    headers = Keyword.get(opts, :headers, [])

    message = encode_event(event)
    publish_opts = build_publish_opts(headers)

    case AMQP.Basic.publish(state.channel, exchange, routing_key, message, publish_opts) do
      :ok ->
        {:reply, :ok, %{state | published_count: state.published_count + 1}}

      {:error, reason} = error ->
        Logger.error("[RabbitMQ.Producer] Publish failed: #{inspect(reason)}")
        {:reply, error, %{state | error_count: state.error_count + 1}}
    end
  end

  def handle_call({:publish, _event, _opts}, _from, state) do
    {:reply, {:error, {:not_connected, state.status}}, state}
  end

  @impl true
  def handle_call({:publish_batch, events, opts}, _from, %{status: :connected} = state) do
    exchange = Keyword.get(opts, :exchange, state.exchange)
    headers = Keyword.get(opts, :headers, [])
    publish_opts = build_publish_opts(headers)

    results =
      Enum.map(events, fn event ->
        routing_key = event[:event_type] || "event"
        message = encode_event(event)
        AMQP.Basic.publish(state.channel, exchange, routing_key, message, publish_opts)
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
      exchange: Map.get(state, :exchange, @default_exchange),
      status: state.status
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

  @impl true
  def handle_info({:DOWN, _ref, :process, _pid, reason}, state) do
    Logger.warning("[RabbitMQ.Producer] Connection lost: #{inspect(reason)}")

    # Attempt to reconnect
    Process.send_after(self(), :reconnect, 5000)

    {:noreply, %{state | status: :disconnected}}
  end

  @impl true
  def handle_info(:reconnect, state) do
    Logger.info("[RabbitMQ.Producer] Attempting to reconnect...")

    case reconnect() do
      {:ok, new_state} ->
        {:noreply, Map.merge(state, new_state)}

      {:error, _reason} ->
        # Retry later
        Process.send_after(self(), :reconnect, 10_000)
        {:noreply, state}
    end
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  @impl true
  def terminate(_reason, %{connection: conn} = _state) do
    if conn do
      AMQP.Connection.close(conn)
    end

    :ok
  end

  def terminate(_reason, _state), do: :ok

  ## Private Functions

  defp reconnect do
    url = get_url()
    exchange = get_exchange()
    exchange_type = get_exchange_type()
    durable = get_durable()

    case AMQP.Connection.open(url) do
      {:ok, conn} ->
        Process.monitor(conn.pid)

        case AMQP.Channel.open(conn) do
          {:ok, channel} ->
            :ok =
              AMQP.Exchange.declare(channel, exchange, exchange_type,
                durable: durable,
                auto_delete: false
              )

            Logger.info("[RabbitMQ.Producer] Reconnected successfully")

            {:ok,
             %{
               connection: conn,
               channel: channel,
               exchange: exchange,
               status: :connected
             }}

          {:error, reason} ->
            AMQP.Connection.close(conn)
            {:error, reason}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp get_url do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    Keyword.get(config, :url) ||
      System.get_env("RABBITMQ_URL", "amqp://guest:guest@localhost:5672")
  end

  defp get_exchange do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :exchange) || System.get_env("RABBITMQ_EXCHANGE", @default_exchange)
  end

  defp get_exchange_type do
    config = Application.get_env(:query_service_ex, __MODULE__, [])

    case Keyword.get(config, :exchange_type) do
      nil ->
        type_str = System.get_env("RABBITMQ_EXCHANGE_TYPE", "topic")
        String.to_atom(type_str)

      type ->
        type
    end
  end

  defp get_durable do
    config = Application.get_env(:query_service_ex, __MODULE__, [])
    Keyword.get(config, :durable, true)
  end

  defp encode_event(event) do
    timestamp = DateTime.utc_now() |> DateTime.to_iso8601()

    message = %{
      event: event,
      metadata: %{
        published_at: timestamp,
        source: "query-service"
      }
    }

    Jason.encode!(message)
  end

  defp build_publish_opts(headers) do
    base_opts = [
      content_type: "application/json",
      persistent: true,
      timestamp: :os.system_time(:second)
    ]

    if Enum.empty?(headers) do
      base_opts
    else
      Keyword.put(base_opts, :headers, headers)
    end
  end

  defp sanitize_url(url) do
    # Hide password in logs
    String.replace(url, ~r/:([^:@]+)@/, ":***@")
  end
end
