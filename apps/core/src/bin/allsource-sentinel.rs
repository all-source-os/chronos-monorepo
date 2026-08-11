//! AllSource Sentinel — Automated Leader Failover
//!
//! Lightweight process that monitors the leader's health and promotes
//! the most up-to-date follower when the leader fails.
//!
//! ## How it works
//!
//! 1. Polls the leader via `GET /health` every `SENTINEL_CHECK_INTERVAL_SECS` seconds.
//! 2. If the leader misses `SENTINEL_FAILURE_THRESHOLD` consecutive heartbeats,
//!    failover begins.
//! 3. Queries all followers for their WAL offset via `GET /health`.
//! 4. Promotes the follower with the highest `last_replayed_offset`.
//! 5. Repoints remaining followers to the new leader.
//! 6. Notifies the query service by updating its `CORE_WRITE_URL` config.
//!
//! ## Environment Variables
//!
//! - `SENTINEL_LEADER_URL`          — Leader HTTP base URL (e.g. `http://core-leader:3900`)
//! - `SENTINEL_FOLLOWER_URLS`       — Comma-separated follower HTTP base URLs
//! - `SENTINEL_QUERY_SERVICE_URL`   — Query service URL for write-URL update notification (optional)
//! - `SENTINEL_CHECK_INTERVAL_SECS` — Health check interval (default: 10)
//! - `SENTINEL_FAILURE_THRESHOLD`   — Consecutive failures before failover (default: 3)
//! - `SENTINEL_REPLICATION_PORT`    — Replication port on promoted follower (default: 3910)

use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Parsed health response from an AllSource Core node.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
struct HealthResponse {
    status: String,
    role: Option<String>,
    replication: Option<serde_json::Value>,
}

/// A follower node with its URL and latest health info.
#[derive(Debug, Clone)]
struct FollowerNode {
    url: String,
    last_replayed_offset: u64,
    healthy: bool,
}

/// Sentinel configuration from environment variables.
struct SentinelConfig {
    leader_url: String,
    follower_urls: Vec<String>,
    query_service_url: Option<String>,
    check_interval: Duration,
    failure_threshold: u32,
    replication_port: u16,
}

impl SentinelConfig {
    fn from_env() -> Self {
        let leader_url = std::env::var("SENTINEL_LEADER_URL")
            .unwrap_or_else(|_| "http://localhost:3900".to_string());

        let follower_urls: Vec<String> = std::env::var("SENTINEL_FOLLOWER_URLS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let query_service_url = std::env::var("SENTINEL_QUERY_SERVICE_URL")
            .ok()
            .filter(|s| !s.is_empty());

        let check_interval = std::env::var("SENTINEL_CHECK_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10);

        let failure_threshold = std::env::var("SENTINEL_FAILURE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let replication_port = std::env::var("SENTINEL_REPLICATION_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3910);

        Self {
            leader_url,
            follower_urls,
            query_service_url,
            check_interval: Duration::from_secs(check_interval),
            failure_threshold,
            replication_port,
        }
    }
}

/// HTTP client with timeout for health checks and failover commands.
struct SentinelClient {
    client: reqwest::Client,
}

impl SentinelClient {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    /// Check a node's health. Returns None if unreachable.
    async fn check_health(&self, base_url: &str) -> Option<HealthResponse> {
        let url = format!("{}/health", base_url.trim_end_matches('/'));
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => resp.json::<HealthResponse>().await.ok(),
            Ok(resp) => {
                tracing::debug!("{} returned HTTP {}", url, resp.status());
                None
            }
            Err(e) => {
                tracing::debug!("{} unreachable: {}", url, e);
                None
            }
        }
    }

    /// Query a follower's WAL offset from its health response.
    async fn get_follower_info(&self, base_url: &str) -> FollowerNode {
        match self.check_health(base_url).await {
            Some(health) => {
                let offset = extract_follower_offset(&health);
                FollowerNode {
                    url: base_url.to_string(),
                    last_replayed_offset: offset,
                    healthy: health.status == "healthy",
                }
            }
            None => FollowerNode {
                url: base_url.to_string(),
                last_replayed_offset: 0,
                healthy: false,
            },
        }
    }

    /// POST /internal/promote on the target node.
    async fn promote(&self, base_url: &str) -> anyhow::Result<()> {
        let url = format!("{}/internal/promote", base_url.trim_end_matches('/'));
        let resp = self.client.post(&url).send().await?;
        if resp.status().is_success() {
            tracing::info!("Promote succeeded on {}", base_url);
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Promote failed on {base_url}: {body}")
        }
    }

    /// POST /internal/repoint?leader=host:port on the target node.
    async fn repoint(&self, base_url: &str, new_leader: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/internal/repoint?leader={}",
            base_url.trim_end_matches('/'),
            new_leader,
        );
        let resp = self.client.post(&url).send().await?;
        if resp.status().is_success() {
            tracing::info!("Repoint succeeded on {} → {}", base_url, new_leader);
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Repoint failed on {base_url}: {body}")
        }
    }
}

/// Extract the follower's last_replayed_offset from the health response.
fn extract_follower_offset(health: &HealthResponse) -> u64 {
    health
        .replication
        .as_ref()
        .and_then(|r| r.get("last_replayed_offset"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

/// Derive the replication address (host:port) from a follower's HTTP URL.
///
/// Given `http://allsource-core-follower-1:3900`, extracts the host
/// and appends the replication port: `allsource-core-follower-1:3910`.
fn replication_addr_from_url(http_url: &str, replication_port: u16) -> String {
    // Parse the URL to extract the host
    if let Ok(url) = url::Url::parse(http_url)
        && let Some(host) = url.host_str()
    {
        return format!("{host}:{replication_port}");
    }
    // Fallback: try simple parsing
    let stripped = http_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host = stripped.split(':').next().unwrap_or(stripped);
    let host = host.split('/').next().unwrap_or(host);
    format!("{host}:{replication_port}")
}

/// Run the sentinel failover loop.
async fn run_sentinel(config: SentinelConfig) {
    let client = SentinelClient::new();
    let mut consecutive_failures: u32 = 0;
    let mut current_leader_url = config.leader_url.clone();

    tracing::info!(
        "Sentinel started — monitoring leader at {}",
        current_leader_url,
    );
    tracing::info!("  Followers: {:?}", config.follower_urls,);
    tracing::info!(
        "  Check interval: {:?}, failure threshold: {}",
        config.check_interval,
        config.failure_threshold,
    );

    loop {
        tokio::time::sleep(config.check_interval).await;

        // Check leader health
        match client.check_health(&current_leader_url).await {
            Some(health) if health.status == "healthy" => {
                if consecutive_failures > 0 {
                    tracing::info!(
                        "Leader {} recovered after {} missed heartbeat(s)",
                        current_leader_url,
                        consecutive_failures,
                    );
                }
                consecutive_failures = 0;
                tracing::trace!("Leader {} healthy", current_leader_url);
            }
            Some(health) => {
                consecutive_failures += 1;
                tracing::warn!(
                    "Leader {} unhealthy (status={}), failures: {}/{}",
                    current_leader_url,
                    health.status,
                    consecutive_failures,
                    config.failure_threshold,
                );
            }
            None => {
                consecutive_failures += 1;
                tracing::warn!(
                    "Leader {} unreachable, failures: {}/{}",
                    current_leader_url,
                    consecutive_failures,
                    config.failure_threshold,
                );
            }
        }

        // Check if failover threshold is reached
        if consecutive_failures >= config.failure_threshold {
            tracing::error!(
                "Leader {} failed {} consecutive heartbeats — initiating failover",
                current_leader_url,
                consecutive_failures,
            );

            match perform_failover(&client, &config, &current_leader_url).await {
                Ok(new_leader_url) => {
                    tracing::info!("Failover complete: new leader is {}", new_leader_url,);
                    current_leader_url = new_leader_url;
                    consecutive_failures = 0;
                }
                Err(e) => {
                    tracing::error!("Failover failed: {}. Will retry on next cycle.", e);
                    // Reset counter so we don't spam failover attempts
                    consecutive_failures = 0;
                }
            }
        }
    }
}

/// Perform the failover sequence:
/// 1. Query all followers for their WAL offset
/// 2. Select the follower with the highest offset
/// 3. Promote it to leader
/// 4. Repoint remaining followers to the new leader
/// 5. Notify the query service
///
/// Returns the new leader's HTTP URL on success.
async fn perform_failover(
    client: &SentinelClient,
    config: &SentinelConfig,
    old_leader_url: &str,
) -> anyhow::Result<String> {
    // Step 1: Query all followers
    tracing::info!(
        "Step 1: Querying {} follower(s) for WAL offsets",
        config.follower_urls.len()
    );

    let mut followers: Vec<FollowerNode> = Vec::new();
    for url in &config.follower_urls {
        let info = client.get_follower_info(url).await;
        tracing::info!(
            "  {} — healthy={}, last_replayed_offset={}",
            info.url,
            info.healthy,
            info.last_replayed_offset,
        );
        followers.push(info);
    }

    // Step 2: Select the best candidate (highest offset among healthy followers)
    let candidate = followers
        .iter()
        .filter(|f| f.healthy)
        .max_by_key(|f| f.last_replayed_offset);

    let candidate = match candidate {
        Some(c) => c.clone(),
        None => {
            anyhow::bail!("No healthy followers available for promotion");
        }
    };

    tracing::info!(
        "Step 2: Selected {} for promotion (offset={})",
        candidate.url,
        candidate.last_replayed_offset,
    );

    // Step 3: Promote the selected follower
    tracing::info!("Step 3: Promoting {} to leader", candidate.url);
    client.promote(&candidate.url).await?;

    // Step 4: Repoint remaining followers to the new leader
    let new_leader_replication_addr =
        replication_addr_from_url(&candidate.url, config.replication_port);

    tracing::info!(
        "Step 4: Repointing remaining followers to {}",
        new_leader_replication_addr,
    );

    for follower in &followers {
        if follower.url == candidate.url {
            continue; // Skip the promoted one
        }
        if !follower.healthy {
            tracing::warn!("  Skipping unhealthy follower {}", follower.url);
            continue;
        }
        if let Err(e) = client
            .repoint(&follower.url, &new_leader_replication_addr)
            .await
        {
            tracing::error!("  Failed to repoint {}: {}", follower.url, e);
        }
    }

    // Step 5: Notify the query service (best-effort)
    if let Some(ref qs_url) = config.query_service_url {
        tracing::info!("Step 5: Notifying query service at {}", qs_url);
        notify_query_service(client, qs_url, &candidate.url, old_leader_url).await;
    } else {
        tracing::info!("Step 5: No query service URL configured, skipping notification");
    }

    Ok(candidate.url)
}

/// Notify the query service that the leader has changed.
///
/// This is best-effort — if the query service is unreachable or doesn't
/// support dynamic config updates, the operator can update the env vars
/// and restart it manually.
async fn notify_query_service(
    client: &SentinelClient,
    qs_url: &str,
    new_leader_url: &str,
    old_leader_url: &str,
) {
    // Try to hit the query service config endpoint if it exists.
    // The query service can implement a POST /internal/update-leader endpoint.
    let url = format!("{}/internal/update-leader", qs_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "new_leader_url": new_leader_url,
        "old_leader_url": old_leader_url,
    });

    match client.client.post(&url).json(&body).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(
                "Query service notified: leader changed to {}",
                new_leader_url
            );
        }
        Ok(resp) => {
            tracing::warn!(
                "Query service notification returned HTTP {} — \
                 manual CORE_WRITE_URL update may be needed",
                resp.status(),
            );
        }
        Err(e) => {
            tracing::warn!(
                "Could not reach query service at {}: {} — \
                 manual CORE_WRITE_URL update may be needed",
                qs_url,
                e,
            );
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "allsource_sentinel=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "AllSource Sentinel v{} starting...",
        env!("CARGO_PKG_VERSION"),
    );

    let config = SentinelConfig::from_env();

    if config.follower_urls.is_empty() {
        tracing::error!("SENTINEL_FOLLOWER_URLS is empty — sentinel has no followers to promote");
        std::process::exit(1);
    }

    run_sentinel(config).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_addr_from_url() {
        assert_eq!(
            replication_addr_from_url("http://allsource-core-follower-1:3900", 3910),
            "allsource-core-follower-1:3910",
        );
        assert_eq!(
            replication_addr_from_url("http://10.0.0.5:3900", 3910),
            "10.0.0.5:3910",
        );
        assert_eq!(
            replication_addr_from_url("http://follower:3900/", 3910),
            "follower:3910",
        );
    }

    #[test]
    fn test_extract_follower_offset() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            role: Some("follower".to_string()),
            replication: Some(serde_json::json!({
                "connected": true,
                "leader": "core-leader:3910",
                "last_replayed_offset": 42,
                "leader_offset": 50,
            })),
        };
        assert_eq!(extract_follower_offset(&health), 42);
    }

    #[test]
    fn test_extract_follower_offset_missing() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            role: Some("follower".to_string()),
            replication: None,
        };
        assert_eq!(extract_follower_offset(&health), 0);
    }

    #[test]
    #[allow(unsafe_code)]
    fn test_sentinel_config_defaults() {
        // Clear env vars for test
        // SAFETY: This test runs in isolation; no concurrent threads read these env vars.
        unsafe {
            std::env::remove_var("SENTINEL_LEADER_URL");
            std::env::remove_var("SENTINEL_FOLLOWER_URLS");
            std::env::remove_var("SENTINEL_CHECK_INTERVAL_SECS");
            std::env::remove_var("SENTINEL_FAILURE_THRESHOLD");
            std::env::remove_var("SENTINEL_REPLICATION_PORT");
        }

        let config = SentinelConfig::from_env();
        assert_eq!(config.leader_url, "http://localhost:3900");
        assert!(config.follower_urls.is_empty());
        assert_eq!(config.check_interval, Duration::from_secs(10));
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.replication_port, 3910);
    }
}
