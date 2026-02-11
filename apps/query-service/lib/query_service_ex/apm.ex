defmodule QueryServiceEx.APM do
  @moduledoc """
  APM (Application Performance Monitoring) integration hooks.

  Provides compatibility with popular APM platforms:
  - DataDog (via Prometheus metrics scraping or DogStatsD)
  - NewRelic (via Prometheus integration or custom events)
  - Honeycomb, Dynatrace, and other OpenTelemetry-compatible systems

  ## Configuration

  Configure APM integration in your config:

      config :query_service_ex, QueryServiceEx.APM,
        enabled: true,
        provider: :datadog,  # or :newrelic, :honeycomb, :custom
        statsd_host: "localhost",
        statsd_port: 8125,
        tags: [
          env: "production",
          service: "query-service"
        ]

  ## DataDog Integration

  For DataDog, the Query Service exposes metrics in Prometheus format at `/metrics`.
  Configure the DataDog Agent to scrape this endpoint:

      # datadog.yaml
      init_config:
      instances:
        - prometheus_url: http://localhost:3902/metrics
          namespace: query_service
          metrics:
            - phoenix_*
            - query_service_ex_*
            - vm_*

  Alternatively, use DogStatsD by setting `provider: :datadog` with statsd_host/port.

  ## NewRelic Integration

  For NewRelic, use the Prometheus OpenMetrics integration:

      # newrelic-infra.yml
      integrations:
        - name: nri-prometheus
          config:
            standalone: false
            urls:
              - http://localhost:3902/metrics
            targets:
              - description: Query Service
                urls:
                  - http://localhost:3902/metrics

  ## Custom APM Events

  This module provides functions to emit custom APM events that can be picked up
  by telemetry handlers:

      # Track a custom business event
      QueryServiceEx.APM.track_event("user.signup", %{plan: "pro"})

      # Track a custom metric
      QueryServiceEx.APM.track_metric("cache.hit_rate", 0.95, tags: [cache: "redis"])

  ## Span/Trace Context

  For distributed tracing, use the span functions to create trace contexts:

      QueryServiceEx.APM.with_span("process_order", fn ->
        # Your code here
        process_order(order_id)
      end)
  """

  require Logger

  @doc """
  Returns APM configuration.
  """
  @spec config() :: Keyword.t()
  def config do
    Application.get_env(:query_service_ex, __MODULE__, [])
  end

  @doc """
  Checks if APM integration is enabled.
  """
  @spec enabled?() :: boolean()
  def enabled? do
    Keyword.get(config(), :enabled, false)
  end

  @doc """
  Returns the configured APM provider.
  """
  @spec provider() :: atom()
  def provider do
    Keyword.get(config(), :provider, :prometheus)
  end

  @doc """
  Returns the service name for APM tagging.
  """
  @spec service_name() :: String.t()
  def service_name do
    Keyword.get(config(), :service_name, "query-service")
  end

  @doc """
  Returns default tags for APM metrics.
  """
  @spec default_tags() :: Keyword.t()
  def default_tags do
    Keyword.get(config(), :tags, []) ++
      [
        service: service_name(),
        runtime: "beam",
        language: "elixir"
      ]
  end

  @doc """
  Tracks a custom event for APM.

  Events are emitted via telemetry and can be picked up by any APM system
  that supports telemetry handlers.

  ## Examples

      iex> APM.track_event("order.completed", %{order_id: "123", total: 99.99})
      :ok

      iex> APM.track_event("user.login", %{user_id: "456"}, tags: [method: :oauth])
      :ok
  """
  @spec track_event(String.t(), map(), Keyword.t()) :: :ok
  def track_event(event_name, attributes \\ %{}, opts \\ []) do
    tags = Keyword.get(opts, :tags, []) ++ default_tags()

    :telemetry.execute(
      [:query_service_ex, :apm, :event],
      %{timestamp: System.system_time(:nanosecond)},
      %{
        event_name: event_name,
        attributes: attributes,
        tags: tags
      }
    )

    :ok
  end

  @doc """
  Tracks a custom metric for APM.

  ## Examples

      iex> APM.track_metric("cache.hit_rate", 0.95)
      :ok

      iex> APM.track_metric("queue.depth", 42, tags: [queue: "events"])
      :ok
  """
  @spec track_metric(String.t(), number(), Keyword.t()) :: :ok
  def track_metric(metric_name, value, opts \\ []) do
    tags = Keyword.get(opts, :tags, []) ++ default_tags()
    metric_type = Keyword.get(opts, :type, :gauge)

    :telemetry.execute(
      [:query_service_ex, :apm, :metric],
      %{value: value},
      %{
        metric_name: metric_name,
        metric_type: metric_type,
        tags: tags
      }
    )

    :ok
  end

  @doc """
  Tracks a histogram/distribution metric.

  ## Examples

      iex> APM.track_histogram("request.size", 1024, unit: :bytes)
      :ok
  """
  @spec track_histogram(String.t(), number(), Keyword.t()) :: :ok
  def track_histogram(metric_name, value, opts \\ []) do
    tags = Keyword.get(opts, :tags, []) ++ default_tags()
    unit = Keyword.get(opts, :unit, :unit)

    :telemetry.execute(
      [:query_service_ex, :apm, :histogram],
      %{value: value},
      %{
        metric_name: metric_name,
        unit: unit,
        tags: tags
      }
    )

    :ok
  end

  @doc """
  Increments a counter metric.

  ## Examples

      iex> APM.increment("requests.total")
      :ok

      iex> APM.increment("events.processed", 10, tags: [type: "user.created"])
      :ok
  """
  @spec increment(String.t(), non_neg_integer(), Keyword.t()) :: :ok
  def increment(metric_name, count \\ 1, opts \\ []) do
    tags = Keyword.get(opts, :tags, []) ++ default_tags()

    :telemetry.execute(
      [:query_service_ex, :apm, :counter],
      %{count: count},
      %{
        metric_name: metric_name,
        tags: tags
      }
    )

    :ok
  end

  @doc """
  Executes a function within a span/trace context.

  The span duration is automatically measured and reported.

  ## Examples

      iex> APM.with_span("process_order", fn -> do_work() end)
      {:ok, result}

      iex> APM.with_span("database.query", [tags: [table: "users"]], fn -> query() end)
      {:ok, result}
  """
  @spec with_span(String.t(), Keyword.t(), (-> any())) :: any()
  def with_span(span_name, opts \\ [], fun) when is_function(fun, 0) do
    tags = Keyword.get(opts, :tags, []) ++ default_tags()
    start_time = System.monotonic_time()
    trace_id = generate_trace_id()
    span_id = generate_span_id()

    :telemetry.execute(
      [:query_service_ex, :apm, :span_start],
      %{start_time: start_time},
      %{
        span_name: span_name,
        trace_id: trace_id,
        span_id: span_id,
        tags: tags
      }
    )

    try do
      result = fun.()

      end_time = System.monotonic_time()
      duration = end_time - start_time

      :telemetry.execute(
        [:query_service_ex, :apm, :span_end],
        %{duration: duration, start_time: start_time, end_time: end_time},
        %{
          span_name: span_name,
          trace_id: trace_id,
          span_id: span_id,
          status: :ok,
          tags: tags
        }
      )

      result
    rescue
      exception ->
        end_time = System.monotonic_time()
        duration = end_time - start_time

        :telemetry.execute(
          [:query_service_ex, :apm, :span_end],
          %{duration: duration, start_time: start_time, end_time: end_time},
          %{
            span_name: span_name,
            trace_id: trace_id,
            span_id: span_id,
            status: :error,
            error: Exception.message(exception),
            tags: tags
          }
        )

        reraise exception, __STACKTRACE__
    end
  end

  @doc """
  Returns APM metadata for distributed tracing.

  This can be used to propagate trace context across service boundaries.

  ## Examples

      iex> APM.trace_context()
      %{trace_id: "abc123", span_id: "def456", sampled: true}
  """
  @spec trace_context() :: map()
  def trace_context do
    %{
      trace_id: generate_trace_id(),
      span_id: generate_span_id(),
      sampled: true,
      service: service_name()
    }
  end

  @doc """
  Generates health check metrics for APM.

  Returns a map of health indicators that APM systems can monitor.
  """
  @spec health_metrics() :: map()
  def health_metrics do
    memory = :erlang.memory()
    {wall_clock_elapsed, _} = :erlang.statistics(:wall_clock)

    %{
      healthy: true,
      uptime_seconds: div(wall_clock_elapsed, 1000),
      memory_bytes: memory[:total],
      process_count: length(Process.list()),
      scheduler_count: :erlang.system_info(:schedulers_online),
      otp_release: :erlang.system_info(:otp_release) |> to_string(),
      elixir_version: System.version()
    }
  end

  # Generate a random trace ID (16 bytes hex-encoded)
  defp generate_trace_id do
    :crypto.strong_rand_bytes(16) |> Base.encode16(case: :lower)
  end

  # Generate a random span ID (8 bytes hex-encoded)
  defp generate_span_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end
end
