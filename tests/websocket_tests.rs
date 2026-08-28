//! tests/websocket_tests.rs
//! Integration tests for WebSocket real-time metrics streaming and live dashboard.
//! Connects to: src/handlers/websocket.rs, src/services/websocket_broadcaster.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::{LatencyTracker, TracingMiddleware};
use fastest_json_api_actix::services::{
    MetricsService, RingBufferService, ShardedCacheService, WebSocketBroadcaster,
};
use std::sync::Arc;

#[actix_web::test]
async fn test_live_dashboard_html_render() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());
    let broadcaster = WebSocketBroadcaster::new(
        Arc::clone(&metrics_service),
        Arc::clone(&ring_buffer_service),
        Arc::clone(&cache_service),
    );

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .app_data(web::Data::new(broadcaster))
            .configure(configure_routes),
    )
    .await;

    // Test /dashboard
    let req = test::TestRequest::get().uri("/dashboard").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let content_type = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(content_type.contains("text/html"));

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Actix Ultra-Fast JSON API"));
    assert!(body_str.contains("/ws/metrics"));
    assert!(body_str.contains("sendCommand('ping')"));

    // Test /api/v1/stream/dashboard
    let req_v1 = test::TestRequest::get()
        .uri("/api/v1/stream/dashboard")
        .to_request();
    let resp_v1 = test::call_service(&app, req_v1).await;
    assert_eq!(resp_v1.status().as_u16(), 200);
}

#[actix_web::test]
async fn test_websocket_broadcaster_frame_construction() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());
    let broadcaster = WebSocketBroadcaster::new(
        Arc::clone(&metrics_service),
        Arc::clone(&ring_buffer_service),
        Arc::clone(&cache_service),
    );

    let frame = broadcaster.build_current_frame(
        &metrics_service,
        &ring_buffer_service,
        &cache_service,
    );

    assert_eq!(frame.total_requests, 0);
    assert_eq!(frame.ring_buffer_occupancy, 0);
    assert_eq!(frame.cache_total_keys, 0);
    assert!(!frame.timestamp.is_empty());
}
