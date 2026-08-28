//! tests/echo_tests.rs
//! Integration tests for JSON echo, serialization speed, and payload processing.
//! Connects to: src/handlers/echo.rs, src/models/echo.rs
//! Created: 2026-08-27

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::EchoResponse;
use fastest_json_api_actix::services::MetricsService;
use serde_json::json;
use std::sync::Arc;

#[actix_web::test]
async fn test_post_echo_success() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    let payload = json!({
        "message": "Actix ultra-fast JSON benchmark payload",
        "count": 100,
        "enabled": true,
        "tags": ["actix", "rust", "low-latency", "microsecond"],
        "metadata": {
            "region": "us-east-1",
            "cluster_id": 42
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/echo")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let echo_resp: EchoResponse = test::read_body_json(resp).await;
    assert_eq!(echo_resp.received.message, "Actix ultra-fast JSON benchmark payload");
    assert_eq!(echo_resp.tag_count, 4);
    assert!(echo_resp.payload_bytes > 0);
}

#[actix_web::test]
async fn test_post_echo_invalid_json() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/v1/echo")
        .insert_header(("content-type", "application/json"))
        .set_payload(r#"{"message": 12345, "invalid": broken"#)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 400);
}
