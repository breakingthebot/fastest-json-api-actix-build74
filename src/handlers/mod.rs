//! src/handlers/mod.rs
//! Router configuration and handler registration.
//! Connects to: src/handlers/*.rs, src/main.rs
//! Created: 2026-08-27

pub mod benchmark;
pub mod cache;
pub mod echo;
pub mod events;
pub mod health;
pub mod metrics;
pub mod ping;
pub mod prometheus;
pub mod rate_limit;
pub mod trace;
pub mod wal;
pub mod websocket;

use actix_web::web;

use crate::handlers::benchmark::{get_synthetic_data, post_ingest_data};
use crate::handlers::cache::{
    delete_cache_key, get_cache_key, get_cache_stats, post_batch_set_cache,
    post_clear_cache, post_purge_expired, put_cache_key,
};
use crate::handlers::echo::post_echo;
use crate::handlers::events::{
    get_buffer_stats, get_recent_events, post_drain_buffer, post_ingest_batch,
    post_ingest_zerocopy,
};
use crate::handlers::health::get_health;
use crate::handlers::metrics::{get_metrics, reset_metrics};
use crate::handlers::ping::get_ping;
use crate::handlers::prometheus::get_prometheus_metrics;
use crate::handlers::rate_limit::{get_ratelimit_stats, post_ratelimit_reset};
use crate::handlers::trace::get_current_trace;
use crate::handlers::wal::{get_wal_stats, post_wal_checkpoint, post_wal_sync};
use crate::handlers::websocket::{get_live_dashboard, ws_metrics_stream};

/// Registers all application endpoints and versioned API scopes onto the Actix service configuration.
///
/// # Arguments
/// * `cfg` - Actix web service configuration reference
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Root level health, ping, dashboard, and Prometheus shortcuts
    cfg.route("/health", web::get().to(get_health));
    cfg.route("/ping", web::get().to(get_ping));
    cfg.route("/metrics", web::get().to(get_prometheus_metrics));
    cfg.route("/dashboard", web::get().to(get_live_dashboard));
    cfg.route("/ws/metrics", web::get().to(ws_metrics_stream));

    // Versioned API v1 scope
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(get_health))
            .route("/ping", web::get().to(get_ping))
            .route("/metrics", web::get().to(get_metrics))
            .route("/metrics/prometheus", web::get().to(get_prometheus_metrics))
            .route("/metrics/reset", web::post().to(reset_metrics))
            .route("/trace/current", web::get().to(get_current_trace))
            .route("/stream/metrics", web::get().to(ws_metrics_stream))
            .route("/stream/dashboard", web::get().to(get_live_dashboard))
            .route("/echo", web::post().to(post_echo))
            .route("/benchmark/synthetic", web::get().to(get_synthetic_data))
            .route("/benchmark/ingest", web::post().to(post_ingest_data))
            // Event Ingestion & Ring Buffer Endpoints
            .route("/events/ingest/zerocopy", web::post().to(post_ingest_zerocopy))
            .route("/events/ingest/batch", web::post().to(post_ingest_batch))
            .route("/events/buffer/stats", web::get().to(get_buffer_stats))
            .route("/events/buffer/recent", web::get().to(get_recent_events))
            .route("/events/buffer/drain", web::post().to(post_drain_buffer))
            // Write-Ahead Log (WAL) Endpoints
            .route("/wal/stats", web::get().to(get_wal_stats))
            .route("/wal/sync", web::post().to(post_wal_sync))
            .route("/wal/checkpoint", web::post().to(post_wal_checkpoint))
            // Rate Limiter Endpoints
            .route("/ratelimit/stats", web::get().to(get_ratelimit_stats))
            .route("/ratelimit/reset", web::post().to(post_ratelimit_reset))
            // 64-Way Sharded Cache Endpoints
            .route("/cache/stats", web::get().to(get_cache_stats))
            .route("/cache/clear", web::post().to(post_clear_cache))
            .route("/cache/purge-expired", web::post().to(post_purge_expired))
            .route("/cache/batch/set", web::post().to(post_batch_set_cache))
            .route("/cache/{key}", web::get().to(get_cache_key))
            .route("/cache/{key}", web::put().to(put_cache_key))
            .route("/cache/{key}", web::delete().to(delete_cache_key)),
    );
}
