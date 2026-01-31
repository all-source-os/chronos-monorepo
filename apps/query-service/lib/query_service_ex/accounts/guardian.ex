defmodule QueryServiceEx.Accounts.Guardian do
  @moduledoc """
  Guardian implementation for JWT authentication.

  Handles encoding user information into JWT tokens and
  decoding tokens back into user resources.
  """

  use Guardian, otp_app: :query_service_ex

  alias QueryServiceEx.Accounts

  @doc """
  Returns a unique identifier for the subject (user) to be encoded in the token.
  """
  @impl Guardian
  def subject_for_token(%{id: id}, _claims) do
    {:ok, to_string(id)}
  end

  def subject_for_token(_, _) do
    {:error, :invalid_resource}
  end

  @doc """
  Returns the resource (user) from the token's subject claim.
  """
  @impl Guardian
  def resource_from_claims(%{"sub" => id}) do
    case Accounts.get_user(id) do
      nil -> {:error, :resource_not_found}
      user -> {:ok, user}
    end
  end

  def resource_from_claims(_claims) do
    {:error, :invalid_claims}
  end
end
