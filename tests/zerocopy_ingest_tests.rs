//! tests/zerocopy_ingest_tests.rs
//! Integration tests for zero-copy and batch event ingestion HTTP endpoints.
//! Connects to: src/handlers/events.rs, src/services/ring_buffer.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::LatencyTracker;
use fastest_json_api_actix::models::{
    BatchIngestResponse, BufferStatsResponse, EventIngestResponse, RecentEventsResponse,
};
use fastest_json_api_actix::services::{MetricsService, RingBufferService};
use serde_json::json;
use std::sync::Arc;

#[actix_web::test]
async fn test_zerocopy_event_ingest_and_query() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .configure(configure_routes),
    )
    .await;

    let payload = json!({
        "event_id": "evt-fast-001",
        "topic": "system.cpu",
        "source": "worker-pool-8",
        "severity": "info",
        "metric_value": 78.4,
        "timestamp_ms": 1724784000000u64
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/events/ingest/zerocopy")
        .insert_header(("content-type", "application/json"))
        .set_payload(payload.to_string())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let ingest_resp: EventIngestResponse = test::read_body_json(resp).await;
    assert_eq!(ingest_resp.status, "success");
    assert_eq!(ingest_resp.assigned_id, 1);
    assert_eq!(ingest_resp.current_buffer_occupancy, 1);

    // Query buffer stats
    let stats_req = test::TestRequest::get()
        .uri("/api/v1/events/buffer/stats")
        .to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    assert_eq!(stats_resp.status().as_u16(), 200);

    let stats: BufferStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.current_occupancy, 1);
    assert_eq!(stats.total_pushed, 1);

    // Query recent events
    let recent_req = test::TestRequest::get()
        .uri("/api/v1/events/buffer/recent?limit=5")
        .to_request();
    let recent_resp = test::call_service(&app, recent_req).await;
    assert_eq!(recent_resp.status().as_u16(), 200);

    let recent: RecentEventsResponse = test::read_body_json(recent_resp).await;
    assert_eq!(recent.count, 1);
    assert_eq!(recent.events[0].event_id, "evt-fast-001");
}

#[actix_web::test]
async fn test_batch_event_ingest_and_drain() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());

    let app = test::init_service(
        App::new()
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .configure(configure_routes),
    )
    .await;

    let batch_payload = json!({
        "batch_id": "batch-integration-01",
        "client_id": "iot-collector",
        "events": [
            {
                "event_id": "e-01",
                "topic": "telemetry",
                "source": "sensor-a",
                "severity": "info",
                "metric_value": 12.3,
                "timestamp_ms": 1724784000000u64
            },
            {
                "event_id": "e-02",
                "topic": "telemetry",
                "source": "sensor-b",
                "severity": "warn",
                "metric_value": 45.6,
                "timestamp_ms": 1724784001000u64
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/api/v1/events/ingest/batch")
        .set_json(&batch_payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let batch_resp: BatchIngestResponse = test::read_body_json(resp).await;
    assert_eq!(batch_resp.status, "success");
    assert_eq!(batch_resp.events_ingested, 2);
    assert_eq!(batch_resp.events_dropped, 0);

    // Drain buffer
    let drain_req = test::TestRequest::post()
        .uri("/api/v1/events/buffer/drain")
        .to_request();
    let drain_resp = test::call_service(&app, drain_req).await;
    assert_eq!(drain_resp.status().as_u16(), 200);

    // Check buffer stats after drain
    let stats_req = test::TestRequest::get()
        .uri("/api/v1/events/buffer/stats")
        .to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    let stats: BufferStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.current_occupancy, 0);
}
