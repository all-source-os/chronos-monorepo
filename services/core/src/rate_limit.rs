/// Rate limiting implementation using token bucket algorithm
///
/// Features:
/// - Per-tenant rate limiting
/// - Per-user rate limiting
/// - Per-API key rate limiting
/// - Configurable limits
/// - Efficient in-memory storage with DashMap
/// - Automatic token replenishment

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

/// Rate limit configuration for different resource types
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per minute
    pub requests_per_minute: u32,
    /// Maximum burst size
    pub burst_size: u32,
}

impl RateLimitConfig {
    /// Free tier: 60 requests/min
    pub fn free_tier() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 100,
        }
    }

    /// Professional tier: 600 requests/min
    pub fn professional() -> Self {
        Self {
            requests_per_minute: 600,
            burst_size: 1000,
        }
    }

    /// Unlimited tier: 10,000 requests/min
    pub fn unlimited() -> Self {
        Self {
            requests_per_minute: 10_000,
            burst_size: 20_000,
        }
    }

    /// Development mode: Very high limits
    pub fn dev_mode() -> Self {
        Self {
            requests_per_minute: 100_000,
            burst_size: 200_000,
        }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Maximum tokens (burst size)
    max_tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: DateTime<Utc>,
}

impl TokenBucket {
    fn new(config: &RateLimitConfig) -> Self {
        let max_tokens = config.burst_size as f64;
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate: config.requests_per_minute as f64 / 60.0, // tokens per second
            last_refill: Utc::now(),
        }
    }

    /// Try to consume a token. Returns true if successful.
    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on time elapsed
    fn refill(&mut self) {
        let now = Utc::now();
        let elapsed = (now - self.last_refill).num_milliseconds() as f64 / 1000.0;

        if elapsed > 0.0 {
            let new_tokens = elapsed * self.refill_rate;
            self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
            self.last_refill = now;
        }
    }

    /// Get remaining tokens
    fn remaining(&mut self) -> u32 {
        self.refill();
        self.tokens.floor() as u32
    }

    /// Get time until next token is available
    fn retry_after(&mut self) -> Duration {
        self.refill();

        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    /// Buckets keyed by identifier (tenant_id, user_id, or api_key_id)
    buckets: Arc<DashMap<String, TokenBucket>>,
    /// Default configuration
    default_config: RateLimitConfig,
    /// Custom configs for specific identifiers
    custom_configs: Arc<DashMap<String, RateLimitConfig>>,
}

impl RateLimiter {
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            default_config,
            custom_configs: Arc::new(DashMap::new()),
        }
    }

    /// Set custom config for a specific identifier
    pub fn set_config(&self, identifier: &str, config: RateLimitConfig) {
        self.custom_configs.insert(identifier.to_string(), config);

        // Reset bucket with new config
        self.buckets.remove(identifier);
    }

    /// Check if request is allowed
    pub fn check_rate_limit(&self, identifier: &str) -> RateLimitResult {
        self.check_rate_limit_with_cost(identifier, 1.0)
    }

    /// Check rate limit with custom cost (for expensive operations)
    pub fn check_rate_limit_with_cost(&self, identifier: &str, cost: f64) -> RateLimitResult {
        let config = self.custom_configs
            .get(identifier)
            .map(|c| c.clone())
            .unwrap_or_else(|| self.default_config.clone());

        let mut entry = self.buckets
            .entry(identifier.to_string())
            .or_insert_with(|| TokenBucket::new(&config));

        let allowed = entry.try_consume(cost);
        let remaining = entry.remaining();
        let retry_after = if !allowed {
            Some(entry.retry_after())
        } else {
            None
        };

        RateLimitResult {
            allowed,
            remaining,
            retry_after,
            limit: config.requests_per_minute,
        }
    }

    /// Get current stats for an identifier
    pub fn get_stats(&self, identifier: &str) -> Option<RateLimitStats> {
        self.buckets.get_mut(identifier).map(|mut bucket| {
            RateLimitStats {
                remaining: bucket.remaining(),
                retry_after: bucket.retry_after(),
            }
        })
    }

    /// Cleanup old buckets (call periodically)
    pub fn cleanup(&self) {
        let now = Utc::now();
        self.buckets.retain(|_, bucket| {
            // Remove buckets that haven't been used in the last hour
            (now - bucket.last_refill).num_hours() < 1
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::professional())
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests
    pub remaining: u32,
    /// Time to wait before retrying (if not allowed)
    pub retry_after: Option<Duration>,
    /// Total limit per minute
    pub limit: u32,
}

/// Current rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    /// Remaining requests
    pub remaining: u32,
    /// Time until next token
    pub retry_after: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_token_bucket_creation() {
        let config = RateLimitConfig::free_tier();
        let bucket = TokenBucket::new(&config);

        assert_eq!(bucket.max_tokens, 100.0);
        assert_eq!(bucket.tokens, 100.0);
    }

    #[test]
    fn test_token_consumption() {
        let config = RateLimitConfig::free_tier();
        let mut bucket = TokenBucket::new(&config);

        assert!(bucket.try_consume(1.0));
        assert_eq!(bucket.remaining(), 99);

        assert!(bucket.try_consume(10.0));
        assert_eq!(bucket.remaining(), 89);
    }

    #[test]
    fn test_rate_limit_enforcement() {
        let config = RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 10,
        };
        let mut bucket = TokenBucket::new(&config);

        // Should allow up to burst size
        for _ in 0..10 {
            assert!(bucket.try_consume(1.0));
        }

        // Should deny after burst exhausted
        assert!(!bucket.try_consume(1.0));
    }

    #[test]
    fn test_token_refill() {
        let config = RateLimitConfig {
            requests_per_minute: 60, // 1 per second
            burst_size: 10,
        };
        let mut bucket = TokenBucket::new(&config);

        // Consume all tokens
        for _ in 0..10 {
            bucket.try_consume(1.0);
        }

        assert_eq!(bucket.remaining(), 0);

        // Wait for refill (simulate)
        thread::sleep(StdDuration::from_secs(2));

        // Should have ~2 tokens refilled
        let remaining = bucket.remaining();
        assert!(remaining >= 1 && remaining <= 3, "Expected 1-3 tokens, got {}", remaining);
    }

    #[test]
    fn test_rate_limiter_per_identifier() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 5,
        });

        // Different identifiers have separate buckets
        let result1 = limiter.check_rate_limit("user1");
        let result2 = limiter.check_rate_limit("user2");

        assert!(result1.allowed);
        assert!(result2.allowed);
        assert_eq!(result1.remaining, 4);
        assert_eq!(result2.remaining, 4);
    }

    #[test]
    fn test_custom_config() {
        let limiter = RateLimiter::new(RateLimitConfig::free_tier());

        limiter.set_config("premium_user", RateLimitConfig::unlimited());

        let free_result = limiter.check_rate_limit("free_user");
        let premium_result = limiter.check_rate_limit("premium_user");

        assert!(free_result.limit < premium_result.limit);
    }

    #[test]
    fn test_rate_limit_with_cost() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_minute: 60,
            burst_size: 10,
        });

        // Expensive operation costs 5 tokens
        let result = limiter.check_rate_limit_with_cost("user1", 5.0);
        assert!(result.allowed);
        assert_eq!(result.remaining, 5);
    }
}
