import Config

# Runtime configuration for production
# All environment variables are optional at startup - the app will fail gracefully
# when trying to use unconfigured services.
if config_env() == :prod do
  # Secret key base - use provided value or generate a temporary one for health checks
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      Base.encode64(:crypto.strong_rand_bytes(64))

  host = System.get_env("PHX_HOST") || "localhost"
  port = String.to_integer(System.get_env("PORT") || "3902")

  config :query_service_ex, QueryServiceExWeb.Endpoint,
    url: [host: host, port: 443, scheme: "https"],
    http: [
      # Enable IPv6 and bind on all interfaces.
      ip: {0, 0, 0, 0, 0, 0, 0, 0},
      port: port
    ],
    secret_key_base: secret_key_base

  # Configure Rust Core backend URLs and authentication
  #
  # For leader-follower replication:
  #   CORE_WRITE_URL  — URL of the leader node (writes always go here)
  #   CORE_READ_URLS  — comma-separated list of all node URLs for round-robin reads
  #
  # Backwards compatible: if neither is set, CORE_URL is used for both reads and writes.
  core_url = System.get_env("CORE_URL") || "http://localhost:3900"

  core_write_url = System.get_env("CORE_WRITE_URL") || core_url

  core_read_urls =
    case System.get_env("CORE_READ_URLS") do
      nil -> [core_url]
      "" -> [core_url]
      urls -> urls |> String.split(",", trim: true) |> Enum.map(&String.trim/1)
    end

  core_max_replication_lag_ms =
    String.to_integer(System.get_env("CORE_MAX_REPLICATION_LAG_MS") || "5000")

  core_health_check_interval_ms =
    String.to_integer(System.get_env("CORE_HEALTH_CHECK_INTERVAL_MS") || "5000")

  config :query_service_ex,
    core_url: core_url,
    core_write_url: core_write_url,
    core_read_urls: core_read_urls,
    core_ws_url: System.get_env("CORE_WS_URL") || "ws://localhost:3900/api/v1/events/stream",
    core_api_key: System.get_env("CORE_API_KEY"),
    core_max_replication_lag_ms: core_max_replication_lag_ms,
    core_health_check_interval_ms: core_health_check_interval_ms

  # LemonSqueezy configuration for billing - optional
  # Variant tier mapping: maps LemonSqueezy variant IDs to Chronos subscription tiers.
  # Set LEMON_SQUEEZY_VARIANT_TIERS as comma-separated "variant_id:tier" pairs.
  # Example: LEMON_SQUEEZY_VARIANT_TIERS="12345:starter,67890:pro,11111:enterprise"
  variant_tiers =
    case System.get_env("LEMON_SQUEEZY_VARIANT_TIERS") do
      nil ->
        %{}

      tiers_string ->
        tiers_string
        |> String.split(",", trim: true)
        |> Enum.reduce(%{}, fn pair, acc ->
          case String.split(String.trim(pair), ":", parts: 2) do
            [variant_id, tier] ->
              Map.put(acc, String.trim(variant_id), String.to_existing_atom(String.trim(tier)))

            _ ->
              acc
          end
        end)
    end

  config :query_service_ex, :lemon_squeezy,
    api_key: System.get_env("LEMON_SQUEEZY_API_KEY"),
    store_id: System.get_env("LEMON_SQUEEZY_STORE_ID"),
    webhook_secret: System.get_env("LEMON_SQUEEZY_WEBHOOK_SECRET"),
    variant_tiers: variant_tiers
end
