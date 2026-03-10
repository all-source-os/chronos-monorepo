defmodule QueryServiceEx.MixProject do
  use Mix.Project

  def project do
    [
      app: :query_service_ex,
      version: "0.14.6",
      elixir: "~> 1.17",
      elixirc_paths: elixirc_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      releases: releases(),
      aliases: aliases(),
      test_coverage: [tool: ExCoveralls],
      dialyzer: dialyzer()
    ]
  end

  defp dialyzer do
    [
      plt_file: {:no_warn, "priv/plts/dialyzer.plt"},
      plt_add_apps: [:mix, :ex_unit],
      ignore_warnings: ".dialyzer_ignore.exs"
    ]
  end

  def cli do
    [
      preferred_envs: [
        coveralls: :test,
        "coveralls.detail": :test,
        "coveralls.post": :test,
        "coveralls.html": :test
      ]
    ]
  end

  # Specifies which paths to compile per environment
  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger],
      mod: {QueryServiceEx.Application, []}
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      # HTTP Client for Rust Core
      {:tesla, "~> 1.11"},
      {:hackney, "~> 1.20"},
      {:jason, "~> 1.4"},

      # WebSocket Client for real-time event streaming from Core
      {:mint_web_socket, "~> 1.0"},
      {:mint, "~> 1.0"},

      # Redis for caching
      {:redix, "~> 1.5"},
      {:castore, "~> 1.0"},

      # Event Processing Pipelines
      {:broadway, "~> 1.1"},
      {:gen_stage, "~> 1.2"},

      # Message Queue Integration
      {:broadway_kafka, "~> 0.4"},
      {:broadway_rabbitmq, "~> 0.8"},

      # Distributed Mode / Clustering
      {:libcluster, "~> 3.3"},
      {:horde, "~> 0.9"},

      # Phoenix for API endpoints
      {:phoenix, "~> 1.7"},
      {:bandit, "~> 1.0"},
      {:cors_plug, "~> 3.0"},

      # OpenAPI/Swagger documentation
      {:open_api_spex, "~> 3.18"},

      # Telemetry & Observability
      {:telemetry, "~> 1.2"},
      {:telemetry_metrics, "~> 1.0"},
      {:telemetry_poller, "~> 1.1"},
      {:telemetry_metrics_prometheus, "~> 1.1"},

      # Structured Logging
      {:logger_json, "~> 7.0"},

      # JWT Authentication
      {:jose, "~> 1.11"},

      # Development & Testing
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},
      {:mix_audit, "~> 2.1", only: [:dev, :test], runtime: false},

      # Mocking
      {:mox, "~> 1.1", only: :test},

      # Code coverage
      {:excoveralls, "~> 0.18", only: :test}
    ]
  end

  defp releases do
    [
      query_service_ex: [
        include_executables_for: [:unix],
        applications: [runtime_tools: :permanent],
        steps: [:assemble, :tar]
      ]
    ]
  end

  defp aliases do
    [
      setup: ["deps.get"]
    ]
  end
end
