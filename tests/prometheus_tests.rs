//! tests/prometheus_tests.rs
//! Integration tests for the Prometheus / OpenMetrics 0.0.4 text exposition endpoint.
//! Connects to: src/handlers/prometheus.rs, src/services/prometheus.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::{LatencyTracker, TracingMiddleware};
use fastest_json_api_actix::services::{MetricsService, RingBufferService, ShardedCacheService};
use std::sync::Arc;

#[actix_web::test]
async fn test_prometheus_exposition_format() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    // Send a ping to generate telemetry
    let ping_req = test::TestRequest::get().uri("/ping").to_request();
    let _ = test::call_service(&app, ping_req).await;

    // Fetch /metrics
    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/plain"));
    assert!(content_type.contains("version=0.0.4"));

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify key Prometheus HELP/TYPE declarations
    assert!(body_str.contains("# HELP http_requests_total"));
    assert!(body_str.contains("# TYPE http_requests_total counter"));
    assert!(body_str.contains("http_requests_total{status_family=\"2xx\"}"));

    assert!(body_str.contains("# HELP http_request_duration_seconds"));
    assert!(body_str.contains("# TYPE http_request_duration_seconds summary"));
    assert!(body_str.contains("http_request_duration_seconds{quantile=\"0.5\"}"));

    assert!(body_str.contains("# HELP ring_buffer_occupancy"));
    assert!(body_str.contains("# TYPE ring_buffer_occupancy gauge"));

    assert!(body_str.contains("# HELP cache_hit_ratio_percent"));
    assert!(body_str.contains("# TYPE cache_hit_ratio_percent gauge"));
}

#[actix_web::test]
async fn test_versioned_prometheus_endpoint() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/metrics/prometheus")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);
}
