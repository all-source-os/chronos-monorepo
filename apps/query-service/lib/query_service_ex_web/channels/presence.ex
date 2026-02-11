defmodule QueryServiceExWeb.Presence do
  @moduledoc """
  Phoenix Presence module for tracking connected WebSocket clients.

  Provides real-time awareness of which users are connected to channels,
  enabling features like user lists, online indicators, and connection tracking.

  ## Usage in Channels

      def handle_info(:after_join, socket) do
        {:ok, _} = Presence.track(socket, socket.assigns.user_id, %{
          user_id: socket.assigns.user_id,
          online_at: System.system_time(:second)
        })

        push(socket, "presence_state", Presence.list(socket))
        {:noreply, socket}
      end

  ## Client-side

  Clients receive presence updates via:

  - `presence_state` - Initial state when joining
  - `presence_diff` - Changes (joins/leaves) after initial state

  Example JavaScript:

      import {Presence} from "phoenix"

      const presence = new Presence(channel);

      presence.onSync(() => {
        const users = presence.list();
        console.log("Online users:", users);
      });

      presence.onJoin((id, current, newPres) => {
        console.log("User joined:", id);
      });

      presence.onLeave((id, current, leftPres) => {
        console.log("User left:", id);
      });
  """

  use Phoenix.Presence,
    otp_app: :query_service_ex,
    pubsub_server: QueryServiceEx.PubSub
end
