//! tests/ring_buffer_tests.rs
//! Unit and integration tests for the lock-free in-memory circular ring buffer.
//! Connects to: src/services/ring_buffer.rs, src/models/event.rs
//! Created: 2026-08-28

use fastest_json_api_actix::models::{BatchEventItem, BatchIngestRequest, ZeroCopyEvent};
use fastest_json_api_actix::services::ring_buffer::{RingBufferService, RING_BUFFER_CAPACITY};

#[test]
fn test_ring_buffer_single_push_and_stats() {
    let buffer = RingBufferService::new();

    let event = ZeroCopyEvent {
        event_id: "evt-001",
        topic: "sensor.temp",
        source: "edge-gateway",
        severity: "info",
        metric_value: 23.5,
        timestamp_ms: 1724784000000,
    };

    let id = buffer.push_zerocopy(&event);
    assert_eq!(id, 1);

    let stats = buffer.get_stats();
    assert_eq!(stats.current_occupancy, 1);
    assert_eq!(stats.total_pushed, 1);
    assert_eq!(stats.total_dropped, 0);
    assert_eq!(stats.capacity, RING_BUFFER_CAPACITY);
}

#[test]
fn test_ring_buffer_batch_push_and_recent_query() {
    let buffer = RingBufferService::new();

    let batch = BatchIngestRequest {
        batch_id: "batch-100".to_string(),
        client_id: "integration-tester".to_string(),
        events: vec![
            BatchEventItem {
                event_id: "e1".to_string(),
                topic: "iot.metrics".to_string(),
                source: "sensor-1".to_string(),
                severity: "info".to_string(),
                metric_value: 10.0,
                timestamp_ms: 1724784000000,
            },
            BatchEventItem {
                event_id: "e2".to_string(),
                topic: "iot.alerts".to_string(),
                source: "sensor-2".to_string(),
                severity: "warn".to_string(),
                metric_value: 55.0,
                timestamp_ms: 1724784001000,
            },
            BatchEventItem {
                event_id: "e3".to_string(),
                topic: "iot.metrics".to_string(),
                source: "sensor-3".to_string(),
                severity: "info".to_string(),
                metric_value: 20.0,
                timestamp_ms: 1724784002000,
            },
        ],
    };

    let (ingested, dropped) = buffer.push_batch(batch);
    assert_eq!(ingested, 3);
    assert_eq!(dropped, 0);

    let recent = buffer.get_recent(10, None);
    assert_eq!(recent.len(), 3);
    // Ordered newest first
    assert_eq!(recent[0].event_id, "e3");
    assert_eq!(recent[1].event_id, "e2");
    assert_eq!(recent[2].event_id, "e1");

    // Filter by topic
    let metrics_only = buffer.get_recent(10, Some("iot.metrics"));
    assert_eq!(metrics_only.len(), 2);
    assert_eq!(metrics_only[0].event_id, "e3");
    assert_eq!(metrics_only[1].event_id, "e1");
}

#[test]
fn test_ring_buffer_drain_and_reset() {
    let buffer = RingBufferService::new();

    let event = ZeroCopyEvent {
        event_id: "drain-me",
        topic: "audit",
        source: "auth-svc",
        severity: "info",
        metric_value: 1.0,
        timestamp_ms: 1724784000000,
    };

    buffer.push_zerocopy(&event);
    assert_eq!(buffer.get_stats().current_occupancy, 1);

    let drained = buffer.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(drained[0].event_id, "drain-me");
    assert_eq!(buffer.get_stats().current_occupancy, 0);

    buffer.reset();
    let stats = buffer.get_stats();
    assert_eq!(stats.total_pushed, 0);
    assert_eq!(stats.current_occupancy, 0);
}
