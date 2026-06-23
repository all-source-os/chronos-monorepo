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
  Extracts a summary quantile value.

  Looks for entries with a `quantile` label matching the given value. This is for
  Prometheus *summary* metrics that pre-compute quantiles (and therefore expose a
  `quantile` label). For *histogram* metrics — which expose cumulative `_bucket`
  counts keyed by `le` instead — use `histogram_quantile/3`.
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

  @doc """
  Computes a quantile (e.g. 0.99) from a Prometheus *histogram* by linear
  interpolation across its cumulative `_bucket{le=...}` series.

  `name` is the histogram base name (no `_bucket` suffix); this looks up
  `"\#{name}_bucket"`. Buckets carrying any other labels (e.g. `query_type`) are
  **aggregated across those labels** — the cumulative counts are summed per `le`
  bound, exactly like Prometheus' `histogram_quantile(q, sum by (le) (rate(...)))`.
  This yields a single overall percentile across every series of the histogram.

  Returns the interpolated bound (in the histogram's native unit — seconds for
  `*_duration_seconds`) or `default` when the histogram is absent or empty.

  Interpolation matches Prometheus semantics: find the bucket whose cumulative
  count first reaches `q * total`, then linear-interpolate the position within
  `(previous_le, le]` over `(previous_count, count]`. The lowest bucket
  interpolates from 0. If the target falls in the `+Inf` bucket (i.e. the largest
  finite bound still doesn't cover it), the largest finite bound is returned
  rather than infinity.
  """
  @spec histogram_quantile(map(), String.t(), float(), float()) :: float()
  def histogram_quantile(parsed, name, q, default \\ 0.0)
      when is_float(q) and q >= 0.0 and q <= 1.0 do
    case Map.get(parsed, "#{name}_bucket") do
      entries when is_list(entries) and entries != [] ->
        buckets = aggregate_buckets(entries)

        case buckets do
          [] ->
            default

          _ ->
            total = buckets |> List.last() |> elem(1)

            if total <= 0.0 do
              default
            else
              interpolate_quantile(buckets, q * total, default)
            end
        end

      _ ->
        default
    end
  end

  # Sums cumulative bucket counts per `le` bound (collapsing all other labels),
  # parses `le` into a comparable float (`+Inf` → :infinity sentinel), and returns
  # an ascending list of `{le_float_or_inf, cumulative_count}` tuples.
  defp aggregate_buckets(entries) do
    entries
    |> Enum.reduce(%{}, fn %{labels: labels, value: value}, acc ->
      case Map.get(labels, "le") do
        nil -> acc
        le_str -> Map.update(acc, parse_le(le_str), to_number(value), &(&1 + to_number(value)))
      end
    end)
    |> Enum.sort_by(fn {le, _} -> le end)
  end

  defp parse_le("+Inf"), do: :infinity
  defp parse_le("Inf"), do: :infinity

  defp parse_le(str) do
    case Float.parse(str) do
      {f, _} -> f
      :error -> :infinity
    end
  end

  # Histogram counts are always finite numbers; guard against the parser's
  # :infinity/:nan sentinels just in case a malformed line slips through.
  defp to_number(v) when is_number(v), do: v * 1.0
  defp to_number(_), do: 0.0

  defp interpolate_quantile(buckets, target, default) do
    do_interpolate(buckets, target, 0.0, 0.0, default)
  end

  defp do_interpolate([], _target, _prev_le, _prev_count, default), do: default

  defp do_interpolate([{le, count} | rest], target, prev_le, prev_count, default) do
    cond do
      count < target ->
        # This bucket doesn't yet cover the target; advance. Carry the last finite
        # bound so the +Inf bucket can fall back to it.
        next_prev_le = if le == :infinity, do: prev_le, else: le
        do_interpolate(rest, target, next_prev_le, count, default)

      le == :infinity ->
        # Target lands in the open-ended top bucket — return the largest finite
        # bound rather than infinity (honest upper estimate).
        if prev_le > 0.0, do: prev_le, else: default

      count == prev_count ->
        # No mass added in this band; the value sits at this bound.
        le

      true ->
        # Linear interpolation within (prev_le, le] over (prev_count, count].
        prev_le + (le - prev_le) * ((target - prev_count) / (count - prev_count))
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
