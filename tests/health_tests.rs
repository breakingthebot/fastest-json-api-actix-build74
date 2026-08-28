//! tests/health_tests.rs
//! Integration tests for health check endpoints and system metadata.
//! Connects to: src/handlers/health.rs, src/models/health.rs
//! Created: 2026-08-27

use actix_web::{test, web, App};
use fastest_json_api_actix::config::AppConfig;
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::HealthResponse;
use fastest_json_api_actix::services::MetricsService;
use std::sync::Arc;
use std::time::Instant;

#[actix_web::test]
async fn test_get_health_root_endpoint() {
    let start_time = Instant::now();
    let config = AppConfig::from_env();
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(start_time))
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    // Verify custom high-resolution performance response headers
    let headers = resp.headers();
    assert!(headers.contains_key("x-response-time-microseconds"));
    assert!(headers.contains_key("x-response-time-ms"));
    assert!(headers.contains_key("x-server-timing"));
    assert_eq!(
        headers.get("server").unwrap().to_str().unwrap(),
        "Actix-Rust-UltraFast/0.1.0"
    );

    let body: HealthResponse = test::read_body_json(resp).await;
    assert_eq!(body.status, "healthy");
    assert_eq!(body.service, "fastest-json-api-actix");
    assert!(body.system.num_cpus > 0);
}

#[actix_web::test]
async fn test_get_health_versioned_endpoint() {
    let start_time = Instant::now();
    let config = AppConfig::from_env();
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(start_time))
            .app_data(web::Data::new(config))
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);
    let body: HealthResponse = test::read_body_json(resp).await;
    assert_eq!(body.status, "healthy");
}
