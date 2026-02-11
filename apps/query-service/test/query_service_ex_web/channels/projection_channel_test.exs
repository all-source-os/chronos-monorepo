defmodule QueryServiceExWeb.ProjectionChannelTest do
  use QueryServiceExWeb.ChannelCase

  alias QueryServiceExWeb.ProjectionChannel

  setup do
    {:ok, socket, user, _token} = create_authenticated_socket()
    {:ok, socket: socket, user: user}
  end

  describe "join projections:{name}" do
    test "successfully joins projection channel", %{socket: socket} do
      {:ok, reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      assert reply.status == "subscribed"
      assert reply.projection == "user_stats"
    end

    test "assigns projection_name to socket", %{socket: socket} do
      {:ok, _reply, socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:order_totals")

      assert socket.assigns.projection_name == "order_totals"
    end

    test "receives presence_state after joining", %{socket: socket} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      assert_push("presence_state", _state)
    end
  end

  describe "state_updated broadcasts" do
    test "receives state_updated messages", %{socket: socket} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      state = %{"event_count" => 42, "last_updated" => "2026-02-02T12:00:00Z"}

      broadcast_projection_update("user_stats", "user-123", state, version: 5)

      assert_push("state_updated", update)
      assert update.entity_id == "user-123"
      assert update.state == state
      assert update.version == 5
    end

    test "does not receive updates for other projections", %{socket: socket} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      # Broadcast to different projection
      broadcast_projection_update("order_totals", "order-123", %{"total" => 500})

      refute_push("state_updated", _)
    end
  end

  describe "projection_error broadcasts" do
    test "receives projection_error messages", %{socket: socket} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      broadcast_projection_error("user_stats", "user-456", "Failed to process event",
        event_id: "evt-789"
      )

      assert_push("projection_error", error)
      assert error.entity_id == "user-456"
      assert error.error == "Failed to process event"
      assert error.event_id == "evt-789"
    end
  end

  describe "get_state incoming message" do
    test "returns error for missing entity_id", %{socket: socket} do
      {:ok, _reply, socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      ref = push(socket, "get_state", %{})
      assert_reply(ref, :error, %{reason: "missing_entity_id"})
    end

    test "returns error for non-existent state", %{socket: socket} do
      {:ok, _reply, socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:nonexistent_projection")

      ref = push(socket, "get_state", %{"entity_id" => "user-999"})
      assert_reply(ref, :error, %{reason: "not_found", entity_id: "user-999"})
    end
  end

  describe "presence tracking" do
    test "tracks projection name in presence metadata", %{socket: socket, user: user} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, ProjectionChannel, "projections:user_stats")

      assert_push("presence_state", state)

      user_id_str = to_string(user.id)
      assert Map.has_key?(state, user_id_str)

      metas = state[user_id_str].metas
      assert length(metas) > 0

      [meta | _] = metas
      assert meta.projection == "user_stats"
    end
  end
end
