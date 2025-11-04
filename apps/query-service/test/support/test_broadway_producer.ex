defmodule QueryServiceEx.TestBroadwayProducer do
  @moduledoc """
  A simple test producer for Broadway pipelines.

  Produces events from a GenServer queue for testing.
  """

  use GenStage

  @doc """
  Start the producer.
  """
  def start_link(opts) do
    GenStage.start_link(__MODULE__, opts)
  end

  @doc """
  Push events to the producer for processing.
  """
  def push_events(producer, events) when is_list(events) do
    GenStage.call(producer, {:push_events, events})
  end

  @impl true
  def init(opts) do
    {:producer, %{queue: :queue.new(), demand: 0}, opts}
  end

  @impl true
  def handle_demand(demand, %{queue: queue, demand: pending_demand} = state) do
    total_demand = demand + pending_demand
    {events, new_queue} = take_events(queue, total_demand)

    new_state = %{
      state
      | queue: new_queue,
        demand: total_demand - length(events)
    }

    {:noreply, events, new_state}
  end

  @impl true
  def handle_call({:push_events, events}, _from, %{queue: queue, demand: demand} = state) do
    # Add events to queue
    new_queue = Enum.reduce(events, queue, fn event, q -> :queue.in(event, q) end)

    # Fulfill pending demand if any
    {to_emit, final_queue} = take_events(new_queue, demand)

    new_state = %{
      state
      | queue: final_queue,
        demand: demand - length(to_emit)
    }

    {:reply, :ok, to_emit, new_state}
  end

  defp take_events(queue, count) do
    take_events(queue, count, [])
  end

  defp take_events(queue, 0, acc) do
    {Enum.reverse(acc), queue}
  end

  defp take_events(queue, count, acc) do
    case :queue.out(queue) do
      {{:value, event}, new_queue} ->
        take_events(new_queue, count - 1, [wrap_event(event) | acc])

      {:empty, queue} ->
        {Enum.reverse(acc), queue}
    end
  end

  defp wrap_event(event) do
    %Broadway.Message{
      data: event,
      acknowledger: {__MODULE__, :ack_ref, :ack_data}
    }
  end
end
