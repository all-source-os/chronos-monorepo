defmodule QueryServiceExWeb.UserSocket do
  @moduledoc """
  Phoenix Socket for WebSocket connections to the Query Service.

  Handles authentication via JWT tokens and routes connections to appropriate channels.

  ## Topics

  - `events:all` - All events from the event store
  - `events:{entity_id}` - Events for a specific entity
  - `events:type:{event_type}` - Events of a specific type
  - `projections:{name}` - Real-time projection state updates

  ## Authentication

  Clients must provide a JWT token via the `token` parameter:

      const socket = new Socket("/ws", {params: {token: "your-jwt-token"}});

  ## Example Usage

      import {Socket} from "phoenix"

      const socket = new Socket("/ws", {params: {token: authToken}});
      socket.connect();

      const channel = socket.channel("events:all", {});
      channel.join()
        .receive("ok", resp => console.log("Joined", resp))
        .receive("error", resp => console.log("Join failed", resp));

      channel.on("new_event", event => {
        console.log("Received event:", event);
      });
  """

  use Phoenix.Socket
  require Logger

  alias QueryServiceEx.Accounts.Guardian

  # Channels
  channel("events:*", QueryServiceExWeb.EventChannel)
  channel("projections:*", QueryServiceExWeb.ProjectionChannel)

  @doc """
  Connect callback for authenticating WebSocket connections.

  Expects a `token` parameter containing a valid JWT token.
  Returns `{:ok, socket}` with user and tenant info assigned on success.
  """
  @impl true
  def connect(%{"token" => token}, socket, _connect_info) when is_binary(token) do
    case verify_token(token) do
      {:ok, user} ->
        Logger.info("[UserSocket] User connected",
          user_id: user.id,
          tenant_id: user.tenant_id
        )

        socket =
          socket
          |> assign(:user_id, user.id)
          |> assign(:user, user)
          |> assign(:tenant_id, user.tenant_id)

        {:ok, socket}

      {:error, reason} ->
        Logger.warning("[UserSocket] Connection rejected: #{inspect(reason)}")
        :error
    end
  end

  def connect(_params, _socket, _connect_info) do
    Logger.warning("[UserSocket] Connection rejected: missing token")
    :error
  end

  @doc """
  Returns a unique socket identifier for the user.

  Used for broadcasting to a specific user's connections.
  """
  @impl true
  def id(socket) do
    "user_socket:#{socket.assigns.user_id}"
  end

  # Private functions

  defp verify_token(token) do
    case Guardian.decode_and_verify(token) do
      {:ok, claims} ->
        Guardian.resource_from_claims(claims)

      {:error, reason} ->
        {:error, reason}
    end
  end
end
