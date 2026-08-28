//! tests/metrics_tests.rs
//! Integration tests for atomic metrics telemetry and latency statistics.
//! Connects to: src/handlers/metrics.rs, src/services/metrics_service.rs
//! Created: 2026-08-27

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::MetricsSnapshot;
use fastest_json_api_actix::services::MetricsService;
use std::sync::Arc;

#[actix_web::test]
async fn test_metrics_collection_and_reset() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service.clone()))
            .configure(configure_routes),
    )
    .await;

    // Send multiple ping requests to generate metrics
    for _ in 0..5 {
        let req = test::TestRequest::get().uri("/ping").to_request();
        let _ = test::call_service(&app, req).await;
    }

    // Query metrics
    let metrics_req = test::TestRequest::get().uri("/api/v1/metrics").to_request();
    let metrics_resp = test::call_service(&app, metrics_req).await;

    assert_eq!(metrics_resp.status().as_u16(), 200);
    let snapshot: MetricsSnapshot = test::read_body_json(metrics_resp).await;

    assert!(snapshot.total_requests >= 5);
    assert!(snapshot.status_2xx >= 5);
    assert_eq!(snapshot.status_5xx, 0);
    assert!(snapshot.endpoints.contains_key("/ping"));

    // Reset metrics via API
    let reset_req = test::TestRequest::post().uri("/api/v1/metrics/reset").to_request();
    let reset_resp = test::call_service(&app, reset_req).await;
    assert_eq!(reset_resp.status().as_u16(), 200);

    // The reset request itself completes through middleware, so total_requests should be 1
    let reset_snapshot = metrics_service.get_snapshot();
    assert_eq!(reset_snapshot.total_requests, 1);
    assert_eq!(reset_snapshot.status_2xx, 1);
    assert_eq!(reset_snapshot.status_5xx, 0);

    // Direct service reset clears completely
    metrics_service.reset();
    let zero_snapshot = metrics_service.get_snapshot();
    assert_eq!(zero_snapshot.total_requests, 0);
    assert_eq!(zero_snapshot.status_2xx, 0);
}
