defmodule McpServerElixir.Protocol.ToonEncoder do
  @moduledoc """
  TOON (Token-Oriented Object Notation) encoder for LLM-optimized responses.

  Formats tabular data as TOON for ~50% token reduction compared to JSON.
  Falls back to JSON for non-tabular or complex nested structures.
  """

  @doc """
  Format response data, using TOON for tabular data and JSON otherwise.

  Supports optional format parameter:
  - `:toon` or `"toon"` - Force TOON format
  - `:json` or `"json"` - Force JSON format
  - `nil` - Auto-detect (default: TOON for tabular, JSON for complex)
  """
  def format_response(data, format \\ nil) do
    case format do
      :toon -> encode_toon(data)
      "toon" -> encode_toon(data)
      :json -> encode_json(data)
      "json" -> encode_json(data)
      nil -> auto_format(data)
      _ -> auto_format(data)
    end
  end

  defp auto_format(data) do
    case detect_tabular_structure(data) do
      {:tabular, _} -> encode_toon(data)
      {:mixed, _} -> encode_json(data)
    end
  end

  defp encode_toon(data) do
    case ToonEx.encode(data) do
      {:ok, toon_string} -> toon_string
      {:error, _reason} -> encode_json(data) # Fallback to JSON if TOON encoding fails
    end
  end

  defp encode_json(data) do
    Jason.encode!(data, pretty: true)
  end

  @doc """
  Detect if data structure is suitable for TOON format.

  TOON excels at:
  - Uniform arrays of objects (same fields, primitive values)
  - Tabular data (event lists, timelines, comparisons)

  Returns:
  - `{:tabular, data}` - Suitable for TOON
  - `{:mixed, data}` - Better as JSON
  """
  def detect_tabular_structure(data) when is_map(data) do
    # Check for common tabular patterns
    cond do
      has_uniform_events?(data) -> {:tabular, data}
      has_uniform_items?(data) -> {:tabular, data}
      has_uniform_comparisons?(data) -> {:tabular, data}
      true -> {:mixed, data}
    end
  end

  def detect_tabular_structure(_), do: {:mixed, nil}

  # Check for "events" array with uniform structure
  defp has_uniform_events?(%{"events" => events}) when is_list(events) and length(events) > 0 do
    uniform_event_list?(events)
  end

  defp has_uniform_events?(%{events: events}) when is_list(events) and length(events) > 0 do
    uniform_event_list?(events)
  end

  defp has_uniform_events?(_), do: false

  # Check for "items" array with uniform structure
  defp has_uniform_items?(%{"items" => items}) when is_list(items) and length(items) > 0 do
    uniform_list?(items)
  end

  defp has_uniform_items?(%{items: items}) when is_list(items) and length(items) > 0 do
    uniform_list?(items)
  end

  defp has_uniform_items?(_), do: false

  # Check for comparison arrays (entity comparisons, etc.)
  defp has_uniform_comparisons?(data) when is_map(data) do
    Enum.any?(data, fn
      {_key, value} when is_list(value) and length(value) > 0 ->
        uniform_list?(value)

      _ ->
        false
    end)
  end

  defp has_uniform_comparisons?(_), do: false

  # Check if event list has uniform structure
  defp uniform_event_list?(events) when length(events) > 0 do
    first_event = List.first(events)
    first_keys = extract_keys(first_event) |> Enum.sort()

    Enum.all?(events, fn event ->
      keys = extract_keys(event) |> Enum.sort()
      keys == first_keys and all_primitive_values?(event)
    end)
  end

  defp uniform_event_list?(_), do: false

  # Check if list has uniform structure
  defp uniform_list?(items) when length(items) > 0 do
    first_item = List.first(items)

    case first_item do
      item when is_map(item) ->
        first_keys = extract_keys(item) |> Enum.sort()

        Enum.all?(items, fn item ->
          keys = extract_keys(item) |> Enum.sort()
          keys == first_keys and all_primitive_values?(item)
        end)

      _ ->
        false
    end
  end

  defp uniform_list?(_), do: false

  # Extract keys from map or struct
  defp extract_keys(item) when is_map(item) do
    Map.keys(item)
  end

  defp extract_keys(_), do: []

  # Check if all values are primitive (no nested objects/arrays)
  defp all_primitive_values?(item) when is_map(item) do
    Enum.all?(item, fn {_key, value} ->
      case value do
        v when is_map(v) -> false
        v when is_list(v) -> false
        _ -> true
      end
    end)
  end

  defp all_primitive_values?(_), do: false
end

