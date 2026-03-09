defmodule QueryServiceEx.PrometheusParser do
  @moduledoc """
  Parses Prometheus text exposition format into structured Elixir maps.

  Handles HELP, TYPE, and metric lines. Supports counters, gauges, histograms,
  and summaries. Returns a map of metric names to their parsed values.
  """

  @doc """
  Parses a Prometheus text format string into a map of metric names to values.

  Returns a map where each key is a metric name (string) and the value is a list
  of `%{labels: map, value: float}` entries.

  ## Examples

      iex> PrometheusParser.parse("events_total 42\\n")
      %{"events_total" => [%{labels: %{}, value: 42.0}]}

      iex> PrometheusParser.parse(~s(http_requests_total{method="GET",code="200"} 1027\\n))
      %{"http_requests_total" => [%{labels: %{"method" => "GET", "code" => "200"}, value: 1027.0}]}
  """
  @spec parse(String.t()) :: %{String.t() => [%{labels: map(), value: float()}]}
  def parse(text) when is_binary(text) do
    text
    |> String.split("\n")
    |> Enum.reject(fn line ->
      trimmed = String.trim(line)
      trimmed == "" or String.starts_with?(trimmed, "#")
    end)
    |> Enum.reduce(%{}, fn line, acc ->
      case parse_metric_line(line) do
        {:ok, name, labels, value} ->
          entry = %{labels: labels, value: value}
          Map.update(acc, name, [entry], &[entry | &1])

        :error ->
          acc
      end
    end)
    |> Map.new(fn {k, v} -> {k, Enum.reverse(v)} end)
  end

  @doc """
  Extracts a single scalar metric value by name (ignoring labels).

  Returns the value of the first entry found, or `default` if not found.
  """
  @spec get_metric(map(), String.t(), float()) :: float()
  def get_metric(parsed, name, default \\ 0.0) do
    case Map.get(parsed, name) do
      [%{value: value} | _] -> value
      _ -> default
    end
  end

  @doc """
  Extracts a histogram quantile value.

  Looks for entries with a `quantile` label matching the given value.
  """
  @spec get_quantile(map(), String.t(), String.t(), float()) :: float()
  def get_quantile(parsed, name, quantile, default \\ 0.0) do
    case Map.get(parsed, name) do
      entries when is_list(entries) ->
        case Enum.find(entries, fn %{labels: labels} ->
               Map.get(labels, "quantile") == quantile
             end) do
          %{value: value} -> value
          nil -> default
        end

      _ ->
        default
    end
  end

  # Parses a single metric line like:
  #   metric_name{label="value"} 123.45
  #   metric_name 123.45
  defp parse_metric_line(line) do
    line = String.trim(line)

    case parse_name_and_labels(line) do
      {name, labels, rest} ->
        case parse_value(rest) do
          {:ok, value} -> {:ok, name, labels, value}
          :error -> :error
        end

      :error ->
        :error
    end
  end

  defp parse_name_and_labels(line) do
    case String.split(line, "{", parts: 2) do
      [name_and_value] ->
        # No labels: "metric_name 123.45" or "metric_name 123.45 timestamp"
        case String.split(String.trim(name_and_value), ~r/\s+/, parts: 2) do
          [name, rest] -> {name, %{}, rest}
          _ -> :error
        end

      [name, labels_and_rest] ->
        case String.split(labels_and_rest, "}", parts: 2) do
          [labels_str, rest] ->
            labels = parse_labels(labels_str)
            {String.trim(name), labels, String.trim(rest)}

          _ ->
            :error
        end
    end
  end

  defp parse_labels(labels_str) do
    labels_str
    |> String.split(",")
    |> Enum.reduce(%{}, fn pair, acc ->
      case String.split(String.trim(pair), "=", parts: 2) do
        [key, value] ->
          # Remove surrounding quotes from value
          clean_value = value |> String.trim() |> String.trim("\"")
          Map.put(acc, String.trim(key), clean_value)

        _ ->
          acc
      end
    end)
  end

  # credo:disable-for-next-line Credo.Check.Refactor.CyclomaticComplexity
  defp parse_value(rest) do
    # Value may be followed by a timestamp: "123.45 1625000000"
    value_str =
      rest
      |> String.trim()
      |> String.split(~r/\s+/, parts: 2)
      |> List.first()

    case value_str do
      nil ->
        :error

      "" ->
        :error

      "+Inf" ->
        {:ok, :infinity}

      "-Inf" ->
        {:ok, :neg_infinity}

      "NaN" ->
        {:ok, :nan}

      str ->
        case Float.parse(str) do
          {value, _} ->
            {:ok, value}

          :error ->
            case Integer.parse(str) do
              {value, _} -> {:ok, value / 1}
              :error -> :error
            end
        end
    end
  end
end
