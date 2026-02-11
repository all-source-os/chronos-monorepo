defmodule QueryServiceExWeb.ChannelBroadcaster do
  @moduledoc """
  Helper module for broadcasting events and updates to WebSocket channels.

  This module provides functions to broadcast events from internal components
  (like ProjectionSync, Broadway pipelines, etc.) to connected WebSocket clients.

  ## Usage

      # Broadcast an event to all subscribers
      ChannelBroadcaster.broadcast_event(event)

      # Broadcast a projection state update
      ChannelBroadcaster.broadcast_projection_update("user_stats", "user-123", new_state)

      # Broadcast a projection error
      ChannelBroadcaster.broadcast_projection_error("user_stats", "user-123", "Failed to process")
  """

  alias Phoenix.PubSub

  @pubsub QueryServiceEx.PubSub

  @doc """
  Broadcasts an event to all relevant topics.

  The event is broadcast to:
  - `events:all` - All event subscribers
  - `events:{entity_id}` - Entity-specific subscribers
  - `events:type:{event_type}` - Event type subscribers
  """
  @spec broadcast_event(map()) :: :ok
  def broadcast_event(event) when is_map(event) do
    # Broadcast to all events topic
    PubSub.broadcast(@pubsub, "events:all", {:new_event, event})

    # Broadcast to entity-specific topic
    if entity_id = event["entity_id"] || event[:entity_id] do
      PubSub.broadcast(@pubsub, "events:#{entity_id}", {:new_event, event})
    end

    # Broadcast to event-type topic
    if event_type = event["event_type"] || event[:event_type] do
      PubSub.broadcast(@pubsub, "events:type:#{event_type}", {:new_event, event})
    end

    :ok
  end

  @doc """
  Broadcasts a projection state update to subscribers.

  ## Parameters

  - `projection_name` - Name of the projection (e.g., "user_stats")
  - `entity_id` - Entity ID the state belongs to
  - `state` - The updated state map
  - `opts` - Optional metadata:
    - `:version` - State version number
    - `:updated_at` - Timestamp of update
  """
  @spec broadcast_projection_update(String.t(), String.t(), map(), keyword()) :: :ok
  def broadcast_projection_update(projection_name, entity_id, state, opts \\ []) do
    update = %{
      entity_id: entity_id,
      state: state,
      version: opts[:version],
      updated_at: opts[:updated_at] || DateTime.utc_now() |> DateTime.to_iso8601()
    }

    PubSub.broadcast(@pubsub, "projections:#{projection_name}", {:state_updated, update})
  end

  @doc """
  Broadcasts a projection error to subscribers.

  ## Parameters

  - `projection_name` - Name of the projection
  - `entity_id` - Entity ID where the error occurred
  - `error` - Error message or description
  - `opts` - Optional metadata:
    - `:event_id` - ID of the event that caused the error
  """
  @spec broadcast_projection_error(String.t(), String.t(), String.t(), keyword()) :: :ok
  def broadcast_projection_error(projection_name, entity_id, error, opts \\ []) do
    error_msg = %{
      entity_id: entity_id,
      error: error,
      event_id: opts[:event_id],
      occurred_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    PubSub.broadcast(@pubsub, "projections:#{projection_name}", {:projection_error, error_msg})
  end

  @doc """
  Returns the count of subscribers for a given topic.

  Useful for monitoring and debugging.
  """
  @spec subscriber_count(String.t()) :: non_neg_integer()
  def subscriber_count(topic) do
    # Registry.lookup always returns a list (possibly empty)
    Registry.lookup(Phoenix.PubSub.Local, {QueryServiceEx.PubSub, topic})
    |> length()
  rescue
    # Table doesn't exist or other registry error
    _ -> 0
  end
end
