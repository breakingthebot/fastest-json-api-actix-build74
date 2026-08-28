//! tests/ping_tests.rs
//! Integration tests for ultra-low latency ping endpoints.
//! Connects to: src/handlers/ping.rs, src/models/ping.rs
//! Created: 2026-08-27

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::PingResponse;
use fastest_json_api_actix::services::MetricsService;
use std::sync::Arc;

#[actix_web::test]
async fn test_get_ping_endpoint() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/ping").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let headers = resp.headers();
    assert!(headers.contains_key("x-response-time-microseconds"));

    let body: PingResponse = test::read_body_json(resp).await;
    assert_eq!(body.message, "pong");
    assert!(body.timestamp_ms > 0);
    assert!(body.unix_nanos > 0);
}
