//! src/handlers/health.rs
//! System health check and runtime status endpoints.
//! Connects to: src/models/health.rs, src/config/app_config.rs, src/handlers/mod.rs
//! Created: 2026-08-27

use actix_web::{web, HttpResponse, Responder};
use std::time::Instant;

use crate::config::AppConfig;
use crate::models::{HealthResponse, SystemMetadata};

/// Handler for `GET /health` and `GET /api/v1/health`.
///
/// # Arguments
/// * `start_time` - Server initialization timestamp instant
/// * `config` - App configuration state
///
/// # Returns
/// HTTP 200 OK with `HealthResponse` JSON.
pub async fn get_health(
    start_time: web::Data<Instant>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    let uptime = start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        service: "fastest-json-api-actix".to_string(),
        environment: config.environment.clone(),
        uptime_seconds: uptime,
        timestamp: chrono::Utc::now().to_rfc3339(),
        worker_threads: config.workers,
        system: SystemMetadata {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            num_cpus: num_cpus::get(),
        },
    };

    HttpResponse::Ok().json(response)
}
