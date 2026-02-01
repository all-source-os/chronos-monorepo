defmodule QueryServiceEx.AccountsTest do
  @moduledoc """
  Tests for the Accounts context.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.User

  import QueryServiceEx.AuthHelpers

  @moduletag :database

  describe "create_user/1" do
    test "creates a user with valid attributes" do
      attrs = %{
        email: "test@example.com",
        name: "Test User",
        google_id: "google_123"
      }

      assert {:ok, %User{} = user} = Accounts.create_user(attrs)
      assert user.email == "test@example.com"
      assert user.name == "Test User"
      assert user.google_id == "google_123"
    end

    test "returns error with invalid email" do
      attrs = %{email: "invalid", google_id: "google_123"}

      assert {:error, changeset} = Accounts.create_user(attrs)
      assert %{email: ["must have the @ sign and no spaces"]} = errors_on(changeset)
    end

    test "returns error with missing required fields" do
      assert {:error, changeset} = Accounts.create_user(%{})
      errors = errors_on(changeset)
      assert %{email: ["can't be blank"]} = errors
      assert Map.has_key?(errors, :google_id)
    end

    test "returns error with duplicate email" do
      attrs = %{email: "test@example.com", google_id: "google_123"}
      {:ok, _user} = Accounts.create_user(attrs)

      attrs2 = %{email: "test@example.com", google_id: "google_456"}
      assert {:error, changeset} = Accounts.create_user(attrs2)
      assert %{email: ["has already been taken"]} = errors_on(changeset)
    end

    test "returns error with duplicate google_id" do
      attrs = %{email: "test1@example.com", google_id: "google_123"}
      {:ok, _user} = Accounts.create_user(attrs)

      attrs2 = %{email: "test2@example.com", google_id: "google_123"}
      assert {:error, changeset} = Accounts.create_user(attrs2)
      assert %{google_id: ["has already been taken"]} = errors_on(changeset)
    end
  end

  describe "get_user/1" do
    test "returns the user with the given id" do
      {:ok, user} = Accounts.create_user(%{email: "test@example.com", google_id: "google_123"})
      fetched = Accounts.get_user(user.id)
      assert fetched.id == user.id
      assert fetched.email == user.email
      assert fetched.google_id == user.google_id
    end

    test "returns nil for non-existent id" do
      assert Accounts.get_user(Ecto.UUID.generate()) == nil
    end
  end

  describe "get_user_by_email/1" do
    test "returns the user with the given email" do
      {:ok, user} = Accounts.create_user(%{email: "test@example.com", google_id: "google_123"})
      fetched = Accounts.get_user_by_email("test@example.com")
      assert fetched.id == user.id
      assert fetched.email == user.email
      assert fetched.google_id == user.google_id
    end

    test "returns nil for non-existent email" do
      assert Accounts.get_user_by_email("nonexistent@example.com") == nil
    end
  end

  describe "get_user_by_google_id/1" do
    test "returns the user with the given google_id" do
      {:ok, user} = Accounts.create_user(%{email: "test@example.com", google_id: "google_123"})
      fetched = Accounts.get_user_by_google_id("google_123")
      assert fetched.id == user.id
      assert fetched.email == user.email
      assert fetched.google_id == user.google_id
    end

    test "returns nil for non-existent google_id" do
      assert Accounts.get_user_by_google_id("nonexistent") == nil
    end
  end

  describe "find_or_create_from_google/1" do
    test "creates a new user when google_id not found" do
      auth = mock_google_auth(email: "new@example.com", uid: "new_google_id")

      assert {:ok, %User{} = user} = Accounts.find_or_create_from_google(auth)
      assert user.email == "new@example.com"
      assert user.google_id == "new_google_id"
    end

    test "updates existing user when google_id found" do
      {:ok, existing} =
        Accounts.create_user(%{
          email: "old@example.com",
          name: "Old Name",
          google_id: "google_123"
        })

      auth =
        mock_google_auth(
          email: "new@example.com",
          name: "New Name",
          uid: "google_123"
        )

      assert {:ok, %User{} = user} = Accounts.find_or_create_from_google(auth)
      assert user.id == existing.id
      assert user.email == "new@example.com"
      assert user.name == "New Name"
    end
  end

  describe "update_user/2" do
    test "updates user with valid attributes" do
      {:ok, user} = Accounts.create_user(%{email: "test@example.com", google_id: "google_123"})

      assert {:ok, updated} = Accounts.update_user(user, %{name: "Updated Name"})
      assert updated.name == "Updated Name"
      assert updated.email == "test@example.com"
    end
  end

  describe "delete_user/1" do
    test "deletes the user" do
      {:ok, user} = Accounts.create_user(%{email: "test@example.com", google_id: "google_123"})

      assert {:ok, %User{}} = Accounts.delete_user(user)
      assert Accounts.get_user(user.id) == nil
    end
  end
end
