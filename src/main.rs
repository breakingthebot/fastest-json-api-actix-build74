//! src/main.rs
//! Main application entrypoint for the Actix Ultra-Fast JSON API.
//! Connects to: src/config, src/handlers, src/middleware, src/services, src/models
//! Created: 2026-08-27

use actix_cors::Cors;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{web, App, HttpResponse, HttpServer};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;

use crate::config::AppConfig;
use crate::middleware::{LatencyTracker, TracingMiddleware};
use crate::models::ApiErrorResponse;
use crate::services::{MetricsService, RingBufferService, ShardedCacheService};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize structured logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let config = AppConfig::from_env();
    let bind_addr = config.bind_address();
    let max_payload_bytes = config.max_payload_bytes;

    log::info!("==================================================================");
    log::info!("🚀 Starting Actix Ultra-Fast JSON API (Build 74)");
    log::info!("   Target Address : http://{}", bind_addr);
    log::info!("   Worker Threads : {}", config.workers);
    log::info!("   TCP Backlog    : {}", config.backlog);
    log::info!("   Keep-Alive     : {}s", config.keep_alive_secs);
    log::info!("   Max JSON Size  : {} bytes", max_payload_bytes);
    log::info!("   Cache Shards   : 64 Partitioned Lock-Free Shards");
    log::info!("   Tracing        : W3C Trace Context Propagation Enabled");
    log::info!("   Observability  : OpenMetrics / Prometheus 0.0.4 at /metrics");
    log::info!("   Environment    : {}", config.environment);
    log::info!("==================================================================");

    let start_time = Instant::now();
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let server_start_time = web::Data::new(start_time);
    let server_config = web::Data::new(config.clone());
    let shared_metrics = web::Data::new(metrics_service);
    let shared_ring_buffer = web::Data::new(ring_buffer_service);
    let shared_cache = web::Data::new(cache_service);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        // Custom JSON extractor configuration with standard RFC 7807 error format
        let json_config = web::JsonConfig::default()
            .limit(max_payload_bytes)
            .error_handler(|err, req| {
                let status = match &err {
                    JsonPayloadError::OverflowKnownLength { .. }
                    | JsonPayloadError::Overflow { .. } => StatusCode::PAYLOAD_TOO_LARGE,
                    JsonPayloadError::ContentType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    _ => StatusCode::BAD_REQUEST,
                };

                let error_payload = ApiErrorResponse::new(
                    status.as_u16(),
                    "Invalid JSON Payload",
                    err.to_string(),
                    req.path(),
                );

                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::build(status).json(error_payload),
                )
                .into()
            });

        App::new()
            .wrap(cors)
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(json_config)
            .app_data(server_start_time.clone())
            .app_data(server_config.clone())
            .app_data(shared_metrics.clone())
            .app_data(shared_ring_buffer.clone())
            .app_data(shared_cache.clone())
            .configure(handlers::configure_routes)
    })
    .workers(config.workers)
    .backlog(config.backlog as u32)
    .keep_alive(Duration::from_secs(config.keep_alive_secs))
    .bind(&bind_addr)?
    .run()
    .await
}
