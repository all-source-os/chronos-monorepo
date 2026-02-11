defmodule QueryServiceExWeb.UserSocketTest do
  use QueryServiceExWeb.ChannelCase

  alias QueryServiceExWeb.UserSocket

  describe "connect/3" do
    test "connects with valid JWT token" do
      {:ok, _socket, user, _token} = create_authenticated_socket()

      assert user.id
      assert user.tenant_id
    end

    test "assigns user_id and tenant_id on successful connection" do
      {:ok, socket, user, _token} = create_authenticated_socket()

      assert socket.assigns.user_id == user.id
      assert socket.assigns.tenant_id == user.tenant_id
      # Compare user IDs (socket user is reloaded from DB with tenant preloaded)
      assert socket.assigns.user.id == user.id
      assert socket.assigns.user.email == user.email
    end

    test "rejects connection with invalid token" do
      result = create_unauthenticated_socket()

      assert result == :error
    end

    test "rejects connection without token" do
      result = create_socket_without_token()

      assert result == :error
    end

    test "rejects connection with empty token" do
      result = Phoenix.ChannelTest.connect(UserSocket, %{"token" => ""})

      assert result == :error
    end

    test "rejects connection with nil token" do
      result = Phoenix.ChannelTest.connect(UserSocket, %{"token" => nil})

      assert result == :error
    end
  end

  describe "id/1" do
    test "returns socket id based on user_id" do
      {:ok, socket, user, _token} = create_authenticated_socket()

      assert UserSocket.id(socket) == "user_socket:#{user.id}"
    end
  end
end
