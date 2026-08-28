//! tests/benchmark_tests.rs
//! Integration tests for synthetic batch generation and high-throughput ingestion.
//! Connects to: src/handlers/benchmark.rs, src/models/benchmark.rs
//! Created: 2026-08-27

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::{BenchmarkResponse, IngestRequest, IngestResponse};
use fastest_json_api_actix::services::MetricsService;
use std::sync::Arc;

#[actix_web::test]
async fn test_synthetic_data_generation() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    // Test small size
    let req = test::TestRequest::get()
        .uri("/api/v1/benchmark/synthetic?size=small")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let body: BenchmarkResponse = test::read_body_json(resp).await;
    assert_eq!(body.item_count, 10);
    assert_eq!(body.items.len(), 10);

    // Test custom count
    let req_custom = test::TestRequest::get()
        .uri("/api/v1/benchmark/synthetic?count=50")
        .to_request();
    let resp_custom = test::call_service(&app, req_custom).await;
    let body_custom: BenchmarkResponse = test::read_body_json(resp_custom).await;
    assert_eq!(body_custom.item_count, 50);
    assert_eq!(body_custom.items.len(), 50);
}

#[actix_web::test]
async fn test_batch_ingest_data() {
    let metrics_service = Arc::new(MetricsService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .configure(configure_routes),
    )
    .await;

    // Generate 10 synthetic items to send for ingestion
    let req_synth = test::TestRequest::get()
        .uri("/api/v1/benchmark/synthetic?size=small")
        .to_request();
    let resp_synth = test::call_service(&app, req_synth).await;
    let synth_data: BenchmarkResponse = test::read_body_json(resp_synth).await;

    let ingest_payload = IngestRequest {
        batch_id: "batch-test-001".to_string(),
        client_id: "test-runner".to_string(),
        items: synth_data.items,
    };

    let req = test::TestRequest::post()
        .uri("/api/v1/benchmark/ingest")
        .set_json(&ingest_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let ingest_resp: IngestResponse = test::read_body_json(resp).await;
    assert_eq!(ingest_resp.status, "success");
    assert_eq!(ingest_resp.batch_id, "batch-test-001");
    assert_eq!(ingest_resp.items_processed, 10);
    assert!(ingest_resp.total_value_cents > 0);
}
