//! tests/rate_limiter_tests.rs
//! Unit tests for the 64-way sharded Token Bucket rate limiting service.
//! Connects to: src/services/rate_limiter.rs, src/models/rate_limit.rs
//! Created: 2026-08-28

use fastest_json_api_actix::services::RateLimiterService;
use std::thread;
use std::time::Duration;

#[test]
fn test_token_bucket_burst_and_exhaustion() {
    // 5 tokens capacity, 2 tokens/sec refill
    let limiter = RateLimiterService::new(5, 2.0);

    let client = "192.168.1.100";

    // Consume all 5 burst tokens
    for i in 0..5 {
        let dec = limiter.try_acquire(client);
        assert!(dec.allowed, "Request {} should be allowed", i);
        assert_eq!(dec.limit, 5);
        assert_eq!(dec.remaining, 4 - i);
    }

    // 6th request must be rejected
    let rejected = limiter.try_acquire(client);
    assert!(!rejected.allowed, "6th request should be rejected (429)");
    assert_eq!(rejected.remaining, 0);
    assert!(rejected.reset_seconds >= 1);

    let stats = limiter.get_stats();
    assert_eq!(stats.total_evaluated, 6);
    assert_eq!(stats.total_allowed, 5);
    assert_eq!(stats.total_rejected, 1);
    assert_eq!(stats.active_client_buckets, 1);
}

#[test]
fn test_token_bucket_refill_over_time() {
    // 2 tokens capacity, 5 tokens/sec refill
    let limiter = RateLimiterService::new(2, 5.0);
    let client = "10.0.0.1";

    // Consume both tokens
    assert!(limiter.try_acquire(client).allowed);
    assert!(limiter.try_acquire(client).allowed);
    assert!(!limiter.try_acquire(client).allowed);

    // Wait 300ms (should replenish ~1.5 tokens -> 1 token available)
    thread::sleep(Duration::from_millis(300));

    let dec = limiter.try_acquire(client);
    assert!(dec.allowed, "Token should have refilled");

    limiter.reset();
    let stats = limiter.get_stats();
    assert_eq!(stats.total_evaluated, 0);
    assert_eq!(stats.active_client_buckets, 0);
}
