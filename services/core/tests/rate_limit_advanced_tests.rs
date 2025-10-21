/// Advanced rate limit tests for comprehensive coverage

use allsource_core::rate_limit::{RateLimiter, RateLimitConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_token_refill() {
    let config = RateLimitConfig {
        requests_per_minute: 60, // 1 per second
        burst_size: 10,
    };

    let limiter = RateLimiter::new(config);

    // Use up all burst tokens
    for _ in 0..10 {
        let result = limiter.check_rate_limit("test-tenant");
        assert!(result.allowed);
    }

    // Next request should be limited
    let result = limiter.check_rate_limit("test-tenant");
    assert!(!result.allowed);

    // Wait for token refill (1 second = 1 token)
    thread::sleep(Duration::from_millis(1100));

    // Should have 1 new token
    let result = limiter.check_rate_limit("test-tenant");
    assert!(result.allowed);
}

#[test]
fn test_tenant_isolation() {
    let config = RateLimitConfig {
        requests_per_minute: 5,
        burst_size: 5,
    };

    let limiter = RateLimiter::new(config);

    // Exhaust limits for tenant1
    for _ in 0..5 {
        limiter.check_rate_limit("tenant1");
    }

    // tenant1 should be limited
    let result1 = limiter.check_rate_limit("tenant1");
    assert!(!result1.allowed);

    // tenant2 should still have tokens
    let result2 = limiter.check_rate_limit("tenant2");
    assert!(result2.allowed);
}

#[test]
fn test_rate_limit_headers() {
    let config = RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 10,
    };

    let limiter = RateLimiter::new(config);

    let result = limiter.check_rate_limit("test-tenant");

    assert_eq!(result.limit, 60);
    assert_eq!(result.remaining, 9); // Used 1, 9 remaining
    assert!(result.reset > 0);
}

#[test]
fn test_concurrent_rate_limiting() {
    let config = RateLimitConfig {
        requests_per_minute: 100,
        burst_size: 50,
    };

    let limiter = Arc::new(RateLimiter::new(config));
    let mut handles = vec![];

    // Spawn 10 threads, each making 5 requests
    for i in 0..10 {
        let limiter_clone = Arc::clone(&limiter);
        let handle = thread::spawn(move || {
            let tenant = format!("tenant-{}", i);
            for _ in 0..5 {
                limiter_clone.check_rate_limit(&tenant);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // All tenants should have used 5 requests each
    for i in 0..10 {
        let tenant = format!("tenant-{}", i);
        let result = limiter.check_rate_limit(&tenant);
        // Should still be allowed (used 5, have 45 remaining)
        assert!(result.allowed);
    }
}

#[test]
fn test_custom_tier_config() {
    let limiter = RateLimiter::new(RateLimitConfig {
        requests_per_minute: 10,
        burst_size: 5,
    });

    // Set custom config for premium tenant
    limiter.set_tenant_config(
        "premium-tenant",
        RateLimitConfig {
            requests_per_minute: 1000,
            burst_size: 100,
        },
    );

    // Premium tenant should have higher limits
    let premium_result = limiter.check_rate_limit("premium-tenant");
    assert_eq!(premium_result.limit, 1000);
    assert_eq!(premium_result.remaining, 99);

    // Regular tenant should have default limits
    let regular_result = limiter.check_rate_limit("regular-tenant");
    assert_eq!(regular_result.limit, 10);
    assert_eq!(regular_result.remaining, 4);
}

#[test]
fn test_zero_burst_behavior() {
    let config = RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 0, // No burst allowed
    };

    let limiter = RateLimiter::new(config);

    // First request should be limited (no burst)
    let result = limiter.check_rate_limit("test-tenant");
    assert!(!result.allowed);
}

#[test]
fn test_very_high_rate() {
    let config = RateLimitConfig {
        requests_per_minute: 60000, // 1000 per second
        burst_size: 1000,
    };

    let limiter = RateLimiter::new(config);

    // Should handle high rate without issues
    for _ in 0..1000 {
        let result = limiter.check_rate_limit("high-rate-tenant");
        assert!(result.allowed);
    }

    // 1001st should be limited
    let result = limiter.check_rate_limit("high-rate-tenant");
    assert!(!result.allowed);
}

#[test]
fn test_rate_limit_recovery() {
    let config = RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 10,
    };

    let limiter = RateLimiter::new(config);

    // Exhaust all tokens
    for _ in 0..10 {
        limiter.check_rate_limit("test-tenant");
    }

    // Should be limited
    assert!(!limiter.check_rate_limit("test-tenant").allowed);

    // Wait for full recovery (60 tokens per minute = 1 per second)
    thread::sleep(Duration::from_millis(2000));

    // Should have recovered 2 tokens
    assert!(limiter.check_rate_limit("test-tenant").allowed);
    assert!(limiter.check_rate_limit("test-tenant").allowed);
}

#[test]
fn test_retry_after() {
    let config = RateLimitConfig {
        requests_per_minute: 60,
        burst_size: 10,
    };

    let limiter = RateLimiter::new(config);

    // Exhaust tokens
    for _ in 0..10 {
        limiter.check_rate_limit("test-tenant");
    }

    // Get retry_after
    let result = limiter.check_rate_limit("test-tenant");
    assert!(!result.allowed);
    assert!(result.retry_after.is_some());

    let retry_after = result.retry_after.unwrap();
    assert!(retry_after > 0);
}
