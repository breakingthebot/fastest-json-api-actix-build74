//! src/handlers/events.rs
//! High-throughput zero-copy and batch event ingestion endpoints with WAL persistence.
//! Connects to: src/models/event.rs, src/services/ring_buffer.rs, src/services/wal_service.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::models::{
    BatchIngestRequest, BatchIngestResponse, EventIngestResponse, IngestEvent, RecentEventsQuery,
    RecentEventsResponse, ZeroCopyEvent,
};
use crate::services::{RingBufferService, WalService};

/// Handler for `POST /api/v1/events/ingest/zerocopy`.
/// Deserializes directly from raw bytes using string slice borrows and persists to WAL.
///
/// # Arguments
/// * `body` - Raw request byte slice
/// * `ring_buffer` - Shared circular ring buffer service
/// * `wal` - Shared Write-Ahead Log service
///
/// # Returns
/// HTTP 200 OK with `EventIngestResponse` JSON.
pub async fn post_ingest_zerocopy(
    body: web::Bytes,
    ring_buffer: web::Data<Arc<RingBufferService>>,
    wal: web::Data<Arc<WalService>>,
) -> impl Responder {
    let start_time = Instant::now();

    match serde_json::from_slice::<ZeroCopyEvent>(&body) {
        Ok(borrowed_event) => {
            let now_nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let owned_event = IngestEvent::from_borrowed(&borrowed_event, 0, now_nanos);
            let _ = wal.append_event(&owned_event);

            let assigned_id = ring_buffer.push_zerocopy(&borrowed_event);
            let stats = ring_buffer.get_stats();
            let duration_us = start_time.elapsed().as_micros() as u64;

            HttpResponse::Ok().json(EventIngestResponse {
                status: "success".to_string(),
                assigned_id,
                current_buffer_occupancy: stats.current_occupancy,
                duration_us,
            })
        }
        Err(err) => HttpResponse::BadRequest().json(json!({
            "status": 400,
            "error": "Invalid ZeroCopy JSON Payload",
            "message": err.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Handler for `POST /api/v1/events/ingest/batch`.
///
/// # Arguments
/// * `payload` - Batch event ingestion JSON payload
/// * `ring_buffer` - Shared circular ring buffer service
/// * `wal` - Shared Write-Ahead Log service
///
/// # Returns
/// HTTP 200 OK with `BatchIngestResponse` JSON.
pub async fn post_ingest_batch(
    payload: web::Json<BatchIngestRequest>,
    ring_buffer: web::Data<Arc<RingBufferService>>,
    wal: web::Data<Arc<WalService>>,
) -> impl Responder {
    let start_time = Instant::now();
    let batch = payload.into_inner();
    let batch_id = batch.batch_id.clone();

    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let owned_events: Vec<IngestEvent> = batch
        .events
        .iter()
        .enumerate()
        .map(|(i, e)| IngestEvent {
            id: i as u64,
            event_id: e.event_id.clone(),
            topic: e.topic.clone(),
            source: e.source.clone(),
            severity: e.severity.clone(),
            metric_value: e.metric_value,
            timestamp_ms: e.timestamp_ms,
            ingested_at_nanos: now_nanos,
        })
        .collect();

    let _ = wal.append_batch(&owned_events);

    let (events_ingested, events_dropped) = ring_buffer.push_batch(batch);
    let stats = ring_buffer.get_stats();
    let duration_us = start_time.elapsed().as_micros() as u64;

    HttpResponse::Ok().json(BatchIngestResponse {
        status: "success".to_string(),
        batch_id,
        events_ingested,
        events_dropped,
        buffer_occupancy: stats.current_occupancy,
        duration_us,
    })
}

/// Handler for `GET /api/v1/events/buffer/stats`.
///
/// # Arguments
/// * `ring_buffer` - Shared circular ring buffer service
///
/// # Returns
/// HTTP 200 OK with `BufferStatsResponse` JSON.
pub async fn get_buffer_stats(ring_buffer: web::Data<Arc<RingBufferService>>) -> impl Responder {
    let stats = ring_buffer.get_stats();
    HttpResponse::Ok().json(stats)
}

/// Handler for `GET /api/v1/events/buffer/recent`.
///
/// # Arguments
/// * `query` - Recent events query parameters (`limit`, `topic`)
/// * `ring_buffer` - Shared circular ring buffer service
///
/// # Returns
/// HTTP 200 OK with `RecentEventsResponse` JSON.
pub async fn get_recent_events(
    query: web::Query<RecentEventsQuery>,
    ring_buffer: web::Data<Arc<RingBufferService>>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20);
    let events = ring_buffer.get_recent(limit, query.topic.as_deref());
    let stats = ring_buffer.get_stats();

    HttpResponse::Ok().json(RecentEventsResponse {
        count: events.len(),
        total_occupancy: stats.current_occupancy,
        events,
    })
}

/// Handler for `POST /api/v1/events/buffer/drain`.
///
/// # Arguments
/// * `ring_buffer` - Shared circular ring buffer service
///
/// # Returns
/// HTTP 200 OK with drained events count and timestamp.
pub async fn post_drain_buffer(ring_buffer: web::Data<Arc<RingBufferService>>) -> impl Responder {
    let drained = ring_buffer.drain();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "drained_count": drained.len(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
