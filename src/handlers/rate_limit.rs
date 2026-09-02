//! src/handlers/rate_limit.rs
//! Rate limiter telemetry and administration endpoints.
//! Connects to: src/models/rate_limit.rs, src/services/rate_limiter.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;

use crate::services::RateLimiterService;

/// Handler for `GET /api/v1/ratelimit/stats`.
///
/// # Arguments
/// * `rate_limiter` - Shared rate limiter service
///
/// # Returns
/// HTTP 200 OK with `RateLimitStatsResponse` JSON.
pub async fn get_ratelimit_stats(
    rate_limiter: web::Data<Arc<RateLimiterService>>,
) -> impl Responder {
    let stats = rate_limiter.get_stats();
    HttpResponse::Ok().json(stats)
}

/// Handler for `POST /api/v1/ratelimit/reset`.
///
/// # Arguments
/// * `rate_limiter` - Shared rate limiter service
///
/// # Returns
/// HTTP 200 OK confirmation.
pub async fn post_ratelimit_reset(
    rate_limiter: web::Data<Arc<RateLimiterService>>,
) -> impl Responder {
    rate_limiter.reset();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "All client token buckets and rate limiter counters have been reset.",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
