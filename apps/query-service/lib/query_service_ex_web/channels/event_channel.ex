defmodule QueryServiceExWeb.EventChannel do
  @moduledoc """
  Phoenix Channel for real-time event streaming.

  Bridges internal PubSub events from CoreWebSocketClient to external WebSocket clients.

  ## Topics

  - `events:all` - Subscribe to all events
  - `events:{entity_id}` - Subscribe to events for a specific entity (e.g., `events:user-123`)
  - `events:type:{event_type}` - Subscribe to events of a specific type (e.g., `events:type:user.created`)

  ## Messages

  Incoming events are pushed as `new_event` messages:

      channel.on("new_event", event => {
        console.log(event);
        // {
        //   id: "...",
        //   entity_id: "user-123",
        //   event_type: "user.created",
        //   payload: {...},
        //   timestamp: "2026-02-02T12:00:00Z"
        // }
      });

  ## Presence

  This channel supports Phoenix Presence for tracking connected clients:

      channel.on("presence_state", state => {
        console.log("Current users:", state);
      });

      channel.on("presence_diff", diff => {
        console.log("User joined/left:", diff);
      });
  """

  use Phoenix.Channel
  require Logger

  alias Phoenix.PubSub
  alias QueryServiceExWeb.Presence

  # Intercept presence_diff to handle via handle_out/3
  intercept(["presence_diff"])

  @pubsub QueryServiceEx.PubSub

  @impl true
  def join("events:all", _payload, socket) do
    send(self(), :after_join)
    PubSub.subscribe(@pubsub, "events:all")

    Logger.info("[EventChannel] User joined events:all",
      user_id: socket.assigns.user_id,
      tenant_id: socket.assigns.tenant_id
    )

    emit_channel_joined("events:all")
    {:ok, %{status: "subscribed", topic: "events:all"}, socket}
  end

  def join("events:type:" <> event_type, _payload, socket) do
    send(self(), :after_join)
    topic = "events:type:#{event_type}"
    PubSub.subscribe(@pubsub, topic)

    Logger.info("[EventChannel] User joined #{topic}",
      user_id: socket.assigns.user_id,
      tenant_id: socket.assigns.tenant_id,
      event_type: event_type
    )

    emit_channel_joined(topic)
    {:ok, %{status: "subscribed", topic: topic, event_type: event_type}, socket}
  end

  def join("events:" <> entity_id, _payload, socket) do
    send(self(), :after_join)
    topic = "events:#{entity_id}"
    PubSub.subscribe(@pubsub, topic)

    Logger.info("[EventChannel] User joined #{topic}",
      user_id: socket.assigns.user_id,
      tenant_id: socket.assigns.tenant_id,
      entity_id: entity_id
    )

    emit_channel_joined(topic)
    {:ok, %{status: "subscribed", topic: topic, entity_id: entity_id}, socket}
  end

  @impl true
  def handle_info(:after_join, socket) do
    {:ok, _} =
      Presence.track(socket, socket.assigns.user_id, %{
        user_id: socket.assigns.user_id,
        tenant_id: socket.assigns.tenant_id,
        online_at: System.system_time(:second)
      })

    push(socket, "presence_state", Presence.list(socket))
    {:noreply, socket}
  end

  # Handle PubSub events from CoreWebSocketClient
  def handle_info({:new_event, event}, socket) do
    push(socket, "new_event", event)
    {:noreply, socket}
  end

  # Handle presence_diff broadcasts from Presence tracker
  @impl true
  def handle_out("presence_diff", msg, socket) do
    push(socket, "presence_diff", msg)
    {:noreply, socket}
  end

  @impl true
  def terminate(reason, socket) do
    Logger.info("[EventChannel] User left",
      user_id: socket.assigns.user_id,
      reason: inspect(reason)
    )

    emit_channel_left(socket.topic)
    :ok
  end

  # Telemetry emissions for channel metrics
  defp emit_channel_joined(channel) do
    :telemetry.execute(
      [:query_service_ex, :channel, :joined],
      %{count: 1},
      %{channel: normalize_channel(channel)}
    )
  end

  defp emit_channel_left(channel) do
    :telemetry.execute(
      [:query_service_ex, :channel, :left],
      %{count: 1},
      %{channel: normalize_channel(channel)}
    )
  end

  # Normalize channel names for metrics (avoid high cardinality)
  defp normalize_channel("events:all"), do: "events:all"
  defp normalize_channel("events:type:" <> _), do: "events:type:*"
  defp normalize_channel("events:" <> _), do: "events:entity:*"
  defp normalize_channel(other), do: other
end
