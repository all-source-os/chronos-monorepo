defmodule QueryServiceEx.Application.UseCases.EventPipelineBroadway do
  @moduledoc """
  Broadway-powered event processing pipeline.

  This module integrates our pipeline system with Broadway for high-throughput,
  fault-tolerant event processing with automatic backpressure management.

  ## Features
  - Concurrent event processing with configurable concurrency
  - Automatic batching for efficiency
  - Dead letter queue for failed events
  - Backpressure management
  - Telemetry integration
  - Graceful shutdown and supervision

  ## Example

      defmodule MyApp.OrderPipeline do
        use QueryServiceEx.Application.UseCases.EventPipelineBroadway,
          name: :order_pipeline,
          producer: [
            module: {MyApp.EventProducer, []},
            concurrency: 1
          ],
          processors: [
            default: [concurrency: 10, min_demand: 5, max_demand: 10]
          ],
          batchers: [
            default: [concurrency: 5, batch_size: 100, batch_timeout: 1000]
          ]

        def pipeline do
          Pipeline.Definition.new(
            name: :order_processing,
            version: 1,
            operators: [
              Pipeline.filter_operator("orders_only", fn e ->
                e.event_type == "order.placed"
              end),
              Pipeline.transform_operator("enrich", fn e ->
                # Process event
                e
              end)
            ]
          )
        end
      end

      # Start the pipeline
      {:ok, pid} = MyApp.OrderPipeline.start_link([])
  """

  alias QueryServiceEx.Domain.Entities.Pipeline

  @doc """
  Define a Broadway pipeline with custom configuration.

  Required callbacks:
  - `pipeline/0` - Returns a Pipeline.Definition struct

  Optional callbacks:
  - `handle_failed/2` - Handle failed events (default: log and continue)
  - `handle_batch/3` - Custom batch handling (default: use pipeline)
  """
  defmacro __using__(opts) do
    quote location: :keep, bind_quoted: [opts: opts] do
      use Broadway

      @broadway_opts opts

      @doc """
      Start the Broadway pipeline.
      """
      def start_link(opts \\ []) do
        broadway_opts = Keyword.merge(@broadway_opts, opts)

        Broadway.start_link(__MODULE__,
          name: broadway_opts[:name] || __MODULE__,
          producer: broadway_opts[:producer],
          processors: broadway_opts[:processors] || [default: [concurrency: 10]],
          batchers:
            broadway_opts[:batchers] || [
              default: [concurrency: 5, batch_size: 100, batch_timeout: 1000]
            ],
          context: %{pipeline: pipeline()}
        )
      end

      @doc """
      Handle incoming messages by applying the pipeline.
      """
      @impl Broadway
      def handle_message(_processor, message, %{pipeline: pipeline}) do
        event = message.data

        case Pipeline.apply_pipeline(event, pipeline) do
          {:ok, processed_event} ->
            # Mark message as successful and update data
            message
            |> Broadway.Message.put_data(processed_event)
            |> Broadway.Message.put_batcher(:default)

          :filtered ->
            # Mark as failed with :filtered reason (won't retry)
            Broadway.Message.failed(message, :filtered)

          {:error, reason} ->
            # Mark as failed (will go to handle_failed)
            Broadway.Message.failed(message, reason)
        end
      end

      @doc """
      Handle batches of successfully processed events.

      Default implementation just returns messages unchanged.
      Override for custom batch handling (e.g., bulk database insert).
      """
      @impl Broadway
      def handle_batch(batcher, messages, batch_info, context) do
        # Default: just pass through
        do_handle_batch(batcher, messages, batch_info, context)
      end

      @doc """
      Handle failed messages.

      Default implementation logs the failures.
      Override for custom failure handling (e.g., dead letter queue).
      """
      @impl Broadway
      def handle_failed(messages, context) do
        do_handle_failed(messages, context)
      end

      # Default implementations that can be called by overrides via super()
      defp do_handle_batch(_batcher, messages, _batch_info, _context) do
        # Default: just pass through
        messages
      end

      defp do_handle_failed(messages, _context) do
        # Default implementation: log failures
        Enum.each(messages, fn message ->
          require Logger

          Logger.warning(
            "Event processing failed: #{inspect(message.status.reason)}, data: #{inspect(message.data)}"
          )
        end)

        messages
      end

      # Allow overriding callbacks
      defoverridable handle_batch: 4, handle_failed: 2
    end
  end

  @doc """
  Callback to define the pipeline.

  Must return a `Pipeline.Definition` struct.
  """
  @callback pipeline() :: Pipeline.Definition.t()

  @doc """
  Optional callback to handle batches of processed events.

  Default implementation just returns messages unchanged.
  """
  @callback handle_batch(
              batcher :: atom(),
              messages :: [Broadway.Message.t()],
              batch_info :: Broadway.BatchInfo.t(),
              context :: map()
            ) :: [Broadway.Message.t()]

  @doc """
  Optional callback to handle failed messages.

  Default implementation logs the failure.
  """
  @callback handle_failed(
              messages :: [Broadway.Message.t()],
              context :: map()
            ) :: [Broadway.Message.t()]

  @optional_callbacks handle_batch: 4, handle_failed: 2
end
