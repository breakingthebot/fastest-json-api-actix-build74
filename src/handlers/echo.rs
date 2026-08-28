//! src/handlers/echo.rs
//! JSON echo and serialization benchmarking endpoint handler.
//! Connects to: src/models/echo.rs, src/handlers/mod.rs
//! Created: 2026-08-27

use actix_web::{web, HttpResponse, Responder};
use std::time::Instant;

use crate::models::{EchoRequest, EchoResponse};

/// Handler for `POST /api/v1/echo`.
///
/// # Arguments
/// * `payload` - Parsed JSON body
///
/// # Returns
/// HTTP 200 OK with `EchoResponse` JSON.
pub async fn post_echo(payload: web::Json<EchoRequest>) -> impl Responder {
    let start_time = Instant::now();
    let request_data = payload.into_inner();

    let serialized_bytes = serde_json::to_vec(&request_data).map(|v| v.len()).unwrap_or(0);
    let tag_count = request_data.tags.len();
    let duration_us = start_time.elapsed().as_micros() as u64;

    let response = EchoResponse {
        received: request_data,
        payload_bytes: serialized_bytes,
        tag_count,
        processed_at: chrono::Utc::now().to_rfc3339(),
        server_processing_us: duration_us,
    };

    HttpResponse::Ok().json(response)
}
