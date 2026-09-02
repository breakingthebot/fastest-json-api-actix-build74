//! tests/rate_limit_middleware_tests.rs
//! Integration tests for RateLimitMiddleware, 429 response codes, and rate limit header verification.
//! Connects to: src/middleware/rate_limit_middleware.rs, src/services/rate_limiter.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::{LatencyTracker, RateLimitMiddleware, TracingMiddleware};
use fastest_json_api_actix::models::{RateLimitErrorResponse, RateLimitStatsResponse};
use fastest_json_api_actix::services::{
    MetricsService, RateLimiterService, RingBufferService, ShardedCacheService, WalService,
    WebSocketBroadcaster,
};
use std::fs::remove_file;
use std::path::PathBuf;
use std::sync::Arc;

#[actix_web::test]
async fn test_rate_limit_middleware_throttling_and_bypass() {
    let wal_path = PathBuf::from("target/test_data/rl_test.wal");
    let _ = remove_file(&wal_path);

    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());
    let wal_service = Arc::new(WalService::new(&wal_path).unwrap());
    // Small limit for testing: 3 tokens, 1 token/sec refill
    let rate_limiter = Arc::new(RateLimiterService::new(3, 1.0));
    let broadcaster = WebSocketBroadcaster::new(
        Arc::clone(&metrics_service),
        Arc::clone(&ring_buffer_service),
        Arc::clone(&cache_service),
    );

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(RateLimitMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .app_data(web::Data::new(wal_service))
            .app_data(web::Data::new(rate_limiter))
            .app_data(web::Data::new(broadcaster))
            .configure(configure_routes),
    )
    .await;

    // 1. First 3 requests to /api/v1/cache/item:1 should succeed (200 OK)
    for i in 0..3 {
        let req = test::TestRequest::get()
            .uri("/api/v1/cache/test_key")
            .insert_header(("x-forwarded-for", "203.0.113.195"))
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Key missing is 404, but passed through rate limiter
        assert_eq!(resp.status().as_u16(), 404);

        let headers = resp.headers();
        assert_eq!(headers.get("x-ratelimit-limit").unwrap().to_str().unwrap(), "3");
        assert_eq!(
            headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap(),
            (2 - i).to_string().as_str()
        );
    }

    // 2. 4th request from same IP must return HTTP 429 Too Many Requests
    let blocked_req = test::TestRequest::get()
        .uri("/api/v1/cache/test_key")
        .insert_header(("x-forwarded-for", "203.0.113.195"))
        .to_request();

    let blocked_resp = test::call_service(&app, blocked_req).await;
    assert_eq!(blocked_resp.status().as_u16(), 429);

    let blocked_headers = blocked_resp.headers();
    assert_eq!(blocked_headers.get("x-ratelimit-remaining").unwrap().to_str().unwrap(), "0");
    assert!(blocked_headers.contains_key("retry-after"));

    let err: RateLimitErrorResponse = test::read_body_json(blocked_resp).await;
    assert_eq!(err.status, 429);
    assert_eq!(err.error, "Too Many Requests");

    // 3. Verify /ping and /health bypass rate limiting even when limit is exceeded
    let ping_req = test::TestRequest::get()
        .uri("/ping")
        .insert_header(("x-forwarded-for", "203.0.113.195"))
        .to_request();
    let ping_resp = test::call_service(&app, ping_req).await;
    assert_eq!(ping_resp.status().as_u16(), 200);

    // 4. Query rate limit stats
    let stats_req = test::TestRequest::get()
        .uri("/api/v1/ratelimit/stats")
        .to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    assert_eq!(stats_resp.status().as_u16(), 200);

    let stats: RateLimitStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.burst_capacity, 3);
    assert_eq!(stats.total_evaluated, 4);
    assert_eq!(stats.total_allowed, 3);
    assert_eq!(stats.total_rejected, 1);

    let _ = remove_file(&wal_path);
}
