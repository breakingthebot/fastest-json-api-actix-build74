//! src/handlers/ping.rs
//! High-speed sub-millisecond ping response handler.
//! Connects to: src/models/ping.rs, src/handlers/mod.rs
//! Created: 2026-08-27

use actix_web::{HttpResponse, Responder};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::PingResponse;

/// Handler for `GET /ping` and `GET /api/v1/ping`.
///
/// # Returns
/// HTTP 200 OK with minimal `PingResponse` JSON.
pub async fn get_ping() -> impl Responder {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let response = PingResponse {
        message: "pong".to_string(),
        timestamp_ms: now.as_millis() as i64,
        unix_nanos: now.as_nanos(),
    };

    HttpResponse::Ok().json(response)
}
