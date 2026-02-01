defmodule QueryServiceEx.MixProject do
  use Mix.Project

  def project do
    [
      app: :query_service_ex,
      version: "0.1.0",
      elixir: "~> 1.17",
      elixirc_paths: elixirc_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      releases: releases(),
      aliases: aliases(),
      test_coverage: [tool: ExCoveralls]
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
      {:websockex, "~> 0.4"},

      # Database & State Management
      {:ecto_sql, "~> 3.11"},
      {:postgrex, "~> 0.18"},

      # Redis for caching
      {:redix, "~> 1.5"},

      # Event Processing Pipelines
      {:broadway, "~> 1.1"},
      {:gen_stage, "~> 1.2"},

      # Phoenix for API endpoints
      {:phoenix, "~> 1.7"},
      {:bandit, "~> 1.0"},
      {:cors_plug, "~> 3.0"},

      # Telemetry & Observability
      {:telemetry, "~> 1.2"},
      {:telemetry_metrics, "~> 1.0"},
      {:telemetry_poller, "~> 1.1"},

      # Structured Logging
      {:logger_json, "~> 6.0"},

      # OAuth Authentication
      {:ueberauth, "~> 0.10"},
      {:ueberauth_google, "~> 0.12"},
      {:ueberauth_github, "~> 0.8"},
      {:guardian, "~> 2.3"},

      # Development & Testing
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},

      # Testcontainers for database testing
      {:testcontainers, "~> 1.13", only: :test},

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
      # Setup task runs migrations
      setup: ["deps.get", "ecto.setup"],
      "ecto.setup": ["ecto.create", "ecto.migrate"],
      "ecto.reset": ["ecto.drop", "ecto.setup"]
      # Tests use testcontainers - no ecto.create needed before mix test
    ]
  end
end
