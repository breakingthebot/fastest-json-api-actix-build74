//! tests/wal_tests.rs
//! Unit and integration tests for Write-Ahead Log (WAL) persistence and crash recovery.
//! Connects to: src/services/wal_service.rs, src/handlers/wal.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::{LatencyTracker, TracingMiddleware};
use fastest_json_api_actix::models::{
    IngestEvent, WalCheckpointResponse, WalStatsResponse, WalSyncResponse,
};
use fastest_json_api_actix::services::{
    MetricsService, RingBufferService, ShardedCacheService, WalService, WebSocketBroadcaster,
};
use std::fs::remove_file;
use std::path::PathBuf;
use std::sync::Arc;

#[actix_web::test]
async fn test_wal_append_and_recovery_flow() {
    let temp_wal_path = PathBuf::from("target/test_data/test_events.wal");
    let _ = remove_file(&temp_wal_path);

    // 1. Initialize WAL service and write events
    let wal = WalService::new(&temp_wal_path).expect("Failed to create WAL service");

    let event1 = IngestEvent {
        id: 1,
        event_id: "evt-wal-1".to_string(),
        topic: "orders.created".to_string(),
        source: "pos-1".to_string(),
        severity: "info".to_string(),
        metric_value: 150.0,
        timestamp_ms: 1724784000000,
        ingested_at_nanos: 1724784000000000,
    };

    let event2 = IngestEvent {
        id: 2,
        event_id: "evt-wal-2".to_string(),
        topic: "orders.paid".to_string(),
        source: "pos-1".to_string(),
        severity: "info".to_string(),
        metric_value: 150.0,
        timestamp_ms: 1724784001000,
        ingested_at_nanos: 1724784001000000,
    };

    let bytes1 = wal.append_event(&event1).unwrap();
    let bytes2 = wal.append_event(&event2).unwrap();
    assert!(bytes1 > 0);
    assert!(bytes2 > 0);

    let stats = wal.get_stats();
    assert_eq!(stats.total_appends, 2);
    assert!(stats.file_size_bytes > 0);

    // Sync WAL
    let synced = wal.sync().unwrap();
    assert!(synced > 0);

    // 2. Simulate server crash and restart: Re-open WAL and recover events
    drop(wal);

    let wal_restarted = WalService::new(&temp_wal_path).expect("Failed to reopen WAL");
    let recovered = wal_restarted.recover().expect("Recovery failed");

    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[0].event_id, "evt-wal-1");
    assert_eq!(recovered[1].event_id, "evt-wal-2");

    // 3. Test Checkpoint / Truncation
    let prev_size = wal_restarted.checkpoint().expect("Checkpoint failed");
    assert!(prev_size > 0);
    assert_eq!(wal_restarted.get_stats().file_size_bytes, 0);

    let _ = remove_file(&temp_wal_path);
}

#[actix_web::test]
async fn test_wal_http_api_flow() {
    let temp_wal_path = PathBuf::from("target/test_data/test_http_wal.wal");
    let _ = remove_file(&temp_wal_path);

    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());
    let wal_service = Arc::new(WalService::new(&temp_wal_path).unwrap());
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
            .app_data(web::Data::new(wal_service))
            .app_data(web::Data::new(broadcaster))
            .configure(configure_routes),
    )
    .await;

    // 1. Ingest event via HTTP
    let payload = serde_json::json!({
        "event_id": "evt-http-wal-1",
        "topic": "telemetry.cpu",
        "source": "host-01",
        "severity": "warn",
        "metric_value": 89.5,
        "timestamp_ms": 1724784000000u64
    });

    let ingest_req = test::TestRequest::post()
        .uri("/api/v1/events/ingest/zerocopy")
        .set_json(&payload)
        .to_request();
    let ingest_resp = test::call_service(&app, ingest_req).await;
    assert_eq!(ingest_resp.status().as_u16(), 200);

    // 2. GET /api/v1/wal/stats
    let stats_req = test::TestRequest::get().uri("/api/v1/wal/stats").to_request();
    let stats_resp = test::call_service(&app, stats_req).await;
    assert_eq!(stats_resp.status().as_u16(), 200);

    let stats: WalStatsResponse = test::read_body_json(stats_resp).await;
    assert_eq!(stats.total_appends, 1);
    assert!(stats.file_size_bytes > 0);

    // 3. POST /api/v1/wal/sync
    let sync_req = test::TestRequest::post().uri("/api/v1/wal/sync").to_request();
    let sync_resp = test::call_service(&app, sync_req).await;
    assert_eq!(sync_resp.status().as_u16(), 200);

    let sync_result: WalSyncResponse = test::read_body_json(sync_resp).await;
    assert_eq!(sync_result.status, "success");

    // 4. POST /api/v1/wal/checkpoint
    let cp_req = test::TestRequest::post().uri("/api/v1/wal/checkpoint").to_request();
    let cp_resp = test::call_service(&app, cp_req).await;
    assert_eq!(cp_resp.status().as_u16(), 200);

    let cp_result: WalCheckpointResponse = test::read_body_json(cp_resp).await;
    assert_eq!(cp_result.status, "success");

    let _ = remove_file(&temp_wal_path);
}
