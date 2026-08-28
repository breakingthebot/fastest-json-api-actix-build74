//! tests/cache_api_tests.rs
//! Integration tests for the sharded cache HTTP endpoints.
//! Connects to: src/handlers/cache.rs, src/services/cache_service.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::{
    BatchSetCacheResponse, CacheItemResponse, CacheStatsResponse,
};
use fastest_json_api_actix::services::{MetricsService, RingBufferService, ShardedCacheService};
use serde_json::json;
use std::sync::Arc;

#[actix_web::test]
async fn test_cache_http_crud_flow() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    // 1. PUT /api/v1/cache/user:42
    let put_payload = json!({
        "value": {"username": "rustacean", "tier": "gold"},
        "ttl_seconds": 300
    });

    let put_req = test::TestRequest::put()
        .uri("/api/v1/cache/user:42")
        .set_json(&put_payload)
        .to_request();
    let put_resp = test::call_service(&app, put_req).await;
    assert_eq!(put_resp.status().as_u16(), 200);

    // 2. GET /api/v1/cache/user:42
    let get_req = test::TestRequest::get()
        .uri("/api/v1/cache/user:42")
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status().as_u16(), 200);

    let item: CacheItemResponse = test::read_body_json(get_resp).await;
    assert_eq!(item.key, "user:42");
    assert_eq!(item.value["username"], "rustacean");
    assert_eq!(item.hits, 1);
    assert!(item.ttl_remaining_ms.unwrap() > 0);

    // 3. GET /api/v1/cache/stats
    let stats_req = test::TestRequest::get()
        .uri("/api/v1/cache/stats")
        .to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    let stats: CacheStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.shard_count, 64);
    assert_eq!(stats.total_keys, 1);
    assert_eq!(stats.cache_hits, 1);

    // 4. DELETE /api/v1/cache/user:42
    let del_req = test::TestRequest::delete()
        .uri("/api/v1/cache/user:42")
        .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status().as_u16(), 200);

    // 5. GET /api/v1/cache/user:42 -> 404
    let get_missing = test::TestRequest::get()
        .uri("/api/v1/cache/user:42")
        .to_request();
    let miss_resp = test::call_service(&app, get_missing).await;
    assert_eq!(miss_resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn test_cache_batch_set_and_clear_flow() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    let batch_payload = json!({
        "items": [
            {"key": "product:1", "value": {"title": "Widget A", "price": 9.99}, "ttl_seconds": 600},
            {"key": "product:2", "value": {"title": "Widget B", "price": 19.99}, "ttl_seconds": 600}
        ]
    });

    let batch_req = test::TestRequest::post()
        .uri("/api/v1/cache/batch/set")
        .set_json(&batch_payload)
        .to_request();
    let batch_resp = test::call_service(&app, batch_req).await;
    assert_eq!(batch_resp.status().as_u16(), 200);

    let batch_result: BatchSetCacheResponse = test::read_body_json(batch_resp).await;
    assert_eq!(batch_result.items_set, 2);

    // Clear cache
    let clear_req = test::TestRequest::post()
        .uri("/api/v1/cache/clear")
        .to_request();
    let clear_resp = test::call_service(&app, clear_req).await;
    assert_eq!(clear_resp.status().as_u16(), 200);

    // Verify empty
    let stats_req = test::TestRequest::get()
        .uri("/api/v1/cache/stats")
        .to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    let stats: CacheStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.total_keys, 0);
}
