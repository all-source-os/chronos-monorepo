defmodule QueryServiceExWeb.ChannelCase do
  @moduledoc """
  This module defines the test case to be used by channel tests.

  Such tests rely on `Phoenix.ChannelTest` and also import other
  functionality to make it easier to build channel connections.

  ## Example

      use QueryServiceExWeb.ChannelCase

      setup do
        {:ok, socket, user, token} = create_authenticated_socket()
        {:ok, socket: socket, user: user, token: token}
      end

      test "joining event channel", %{socket: socket} do
        {:ok, _, socket} = subscribe_and_join(socket, EventChannel, "events:all")
        assert socket.assigns.user_id
      end
  """

  use ExUnit.CaseTemplate

  alias Ecto.Adapters.SQL.Sandbox

  using do
    quote do
      # Import conveniences for testing with channels
      import Phoenix.ChannelTest
      import QueryServiceExWeb.ChannelCase

      # Import helper modules
      alias QueryServiceEx.Accounts
      alias QueryServiceEx.Accounts.Guardian
      alias QueryServiceEx.Tenants

      # The default endpoint for testing
      @endpoint QueryServiceExWeb.Endpoint

      @doc """
      Creates an authenticated socket with a test user and tenant.

      Returns `{:ok, socket, user, token}` where:
      - `socket` is a connected Phoenix.Socket
      - `user` is the created user
      - `token` is the JWT token for the user

      ## Options

      - `:email` - Email for the user (default: "test@example.com")
      - `:tenant_name` - Name for the tenant (default: "Test Tenant")
      """
      def create_authenticated_socket(opts \\ []) do
        email = Keyword.get(opts, :email, "test#{:rand.uniform(100_000)}@example.com")
        tenant_name = Keyword.get(opts, :tenant_name, "Test Tenant #{:rand.uniform(100_000)}")

        # Create tenant
        {:ok, tenant} =
          Tenants.create_tenant(%{
            name: tenant_name,
            slug: "test-tenant-#{:rand.uniform(100_000)}",
            subscription_status: :active,
            subscription_tier: :pro
          })

        # Create user
        {:ok, user} =
          Accounts.create_user(%{
            email: email,
            name: "Test User",
            avatar_url: "https://example.com/avatar.jpg",
            google_id: "google_#{:rand.uniform(100_000)}",
            tenant_id: tenant.id
          })

        # Generate token
        {:ok, token, _claims} = Guardian.encode_and_sign(user)

        # Connect socket using the imported macro
        {:ok, socket} = connect(QueryServiceExWeb.UserSocket, %{"token" => token})

        {:ok, socket, user, token}
      end

      @doc """
      Creates a socket that will fail authentication (for testing error cases).
      """
      def create_unauthenticated_socket do
        connect(QueryServiceExWeb.UserSocket, %{"token" => "invalid_token"})
      end

      @doc """
      Creates a socket without a token (for testing missing token case).
      """
      def create_socket_without_token do
        connect(QueryServiceExWeb.UserSocket, %{})
      end
    end
  end

  setup tags do
    :ok = Sandbox.checkout(QueryServiceEx.Repo)

    unless tags[:async] do
      Sandbox.mode(QueryServiceEx.Repo, {:shared, self()})
    end

    :ok
  end

  @doc """
  Broadcasts an event via PubSub (simulates CoreWebSocketClient behavior).

  ## Example

      broadcast_event(%{
        "id" => "event-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{"name" => "John"}
      })
  """
  def broadcast_event(event) do
    Phoenix.PubSub.broadcast(QueryServiceEx.PubSub, "events:all", {:new_event, event})

    if entity_id = event["entity_id"] do
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:#{entity_id}",
        {:new_event, event}
      )
    end

    if event_type = event["event_type"] do
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:type:#{event_type}",
        {:new_event, event}
      )
    end

    :ok
  end

  @doc """
  Broadcasts a projection state update via PubSub.
  """
  def broadcast_projection_update(projection_name, entity_id, state, opts \\ []) do
    update = %{
      entity_id: entity_id,
      state: state,
      version: Keyword.get(opts, :version, 1),
      updated_at: Keyword.get(opts, :updated_at, DateTime.utc_now() |> DateTime.to_iso8601())
    }

    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "projections:#{projection_name}",
      {:state_updated, update}
    )

    :ok
  end

  @doc """
  Broadcasts a projection error via PubSub.
  """
  def broadcast_projection_error(projection_name, entity_id, error, opts \\ []) do
    error_msg = %{
      entity_id: entity_id,
      error: error,
      event_id: Keyword.get(opts, :event_id),
      occurred_at: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    Phoenix.PubSub.broadcast(
      QueryServiceEx.PubSub,
      "projections:#{projection_name}",
      {:projection_error, error_msg}
    )

    :ok
  end
end
