//! src/handlers/wal.rs
//! Write-Ahead Log (WAL) management and persistence endpoints.
//! Connects to: src/models/wal.rs, src/services/wal_service.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

use crate::models::{WalCheckpointResponse, WalSyncResponse};
use crate::services::WalService;

/// Handler for `GET /api/v1/wal/stats`.
///
/// # Arguments
/// * `wal` - Shared Write-Ahead Log service
///
/// # Returns
/// HTTP 200 OK with `WalStatsResponse` JSON.
pub async fn get_wal_stats(wal: web::Data<Arc<WalService>>) -> impl Responder {
    let stats = wal.get_stats();
    HttpResponse::Ok().json(stats)
}

/// Handler for `POST /api/v1/wal/sync`.
/// Forces synchronous flush of page caches to durable physical storage.
///
/// # Arguments
/// * `wal` - Shared Write-Ahead Log service
///
/// # Returns
/// HTTP 200 OK with `WalSyncResponse` JSON.
pub async fn post_wal_sync(wal: web::Data<Arc<WalService>>) -> impl Responder {
    let start_time = Instant::now();
    match wal.sync() {
        Ok(file_size_bytes) => {
            let duration_us = start_time.elapsed().as_micros() as u64;
            HttpResponse::Ok().json(WalSyncResponse {
                status: "success".to_string(),
                duration_us,
                file_size_bytes,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": 500,
            "error": "WAL Sync Failed",
            "message": err.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Handler for `POST /api/v1/wal/checkpoint`.
/// Rotates and truncates the WAL log file after state consolidation.
///
/// # Arguments
/// * `wal` - Shared Write-Ahead Log service
///
/// # Returns
/// HTTP 200 OK with `WalCheckpointResponse` JSON.
pub async fn post_wal_checkpoint(wal: web::Data<Arc<WalService>>) -> impl Responder {
    match wal.checkpoint() {
        Ok(previous_size_bytes) => HttpResponse::Ok().json(WalCheckpointResponse {
            status: "success".to_string(),
            message: format!(
                "WAL checkpoint completed. Truncated from {} bytes to 0 bytes.",
                previous_size_bytes
            ),
            previous_size_bytes,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(err) => HttpResponse::InternalServerError().json(json!({
            "status": 500,
            "error": "WAL Checkpoint Failed",
            "message": err.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}
