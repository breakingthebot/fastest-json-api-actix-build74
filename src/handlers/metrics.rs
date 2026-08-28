//! src/handlers/metrics.rs
//! Endpoints for retrieving real-time server telemetry and latency metrics.
//! Connects to: src/services/metrics_service.rs, src/models/metrics.rs, src/handlers/mod.rs
//! Created: 2026-08-27

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;

use crate::services::MetricsService;

/// Handler for `GET /api/v1/metrics` and `GET /metrics`.
///
/// # Arguments
/// * `metrics_service` - Shared metrics state
///
/// # Returns
/// HTTP 200 OK with `MetricsSnapshot` JSON.
pub async fn get_metrics(metrics_service: web::Data<Arc<MetricsService>>) -> impl Responder {
    let snapshot = metrics_service.get_snapshot();
    HttpResponse::Ok().json(snapshot)
}

/// Handler for `POST /api/v1/metrics/reset`.
///
/// # Arguments
/// * `metrics_service` - Shared metrics state
///
/// # Returns
/// HTTP 200 OK with reset confirmation message.
pub async fn reset_metrics(metrics_service: web::Data<Arc<MetricsService>>) -> impl Responder {
    metrics_service.reset();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "All metrics and latency samples have been reset to zero.",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
