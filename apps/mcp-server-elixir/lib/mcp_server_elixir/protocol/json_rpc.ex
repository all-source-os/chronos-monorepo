defmodule McpServerElixir.Protocol.JsonRpc do
  @moduledoc """
  JSON-RPC 2.0 protocol parser and utilities.

  Handles parsing and validation of JSON-RPC 2.0 messages received over stdio.
  """

  @doc """
  Parse a JSON-RPC 2.0 request from a JSON string.

  Returns:
  - `{:ok, request_map}` - Successfully parsed request
  - `{:error, reason}` - Parse error
  """
  def parse_request(json_string) do
    case Jason.decode(json_string) do
      {:ok, json} ->
        validate_request(json)

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp validate_request(%{"jsonrpc" => "2.0"} = request) do
    # Validate required fields
    cond do
      not Map.has_key?(request, "method") ->
        {:error, "Missing 'method' field"}

      # Request (has id) or notification (no id)
      true ->
        {:ok, normalize_request(request)}
    end
  end

  defp validate_request(_) do
    {:error, "Invalid JSON-RPC version or format"}
  end

  defp normalize_request(request) do
    %{
      method: Map.get(request, "method"),
      params: Map.get(request, "params", %{}),
      id: Map.get(request, "id")
    }
  end
end
