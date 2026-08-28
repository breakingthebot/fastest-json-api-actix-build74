//! src/handlers/prometheus.rs
//! Prometheus / OpenMetrics 0.0.4 text exposition HTTP handler.
//! Connects to: src/services/prometheus.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

use crate::services::{
    render_prometheus_metrics, MetricsService, RingBufferService, ShardedCacheService,
};

/// Handler for `GET /metrics` and `GET /api/v1/metrics/prometheus`.
///
/// # Arguments
/// * `metrics_service` - Shared metrics telemetry service
/// * `ring_buffer` - Shared circular ring buffer service
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with `text/plain; version=0.0.4; charset=utf-8` Prometheus exposition body.
pub async fn get_prometheus_metrics(
    metrics_service: web::Data<Arc<MetricsService>>,
    ring_buffer: web::Data<Arc<RingBufferService>>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> impl Responder {
    let body = render_prometheus_metrics(&metrics_service, &ring_buffer, &cache);

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body)
}
