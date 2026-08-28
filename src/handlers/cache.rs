//! src/handlers/cache.rs
//! HTTP handlers for the 64-way sharded in-memory cache engine.
//! Connects to: src/models/cache.rs, src/services/cache_service.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

use crate::models::{
    BatchSetCacheRequest, BatchSetCacheResponse, CacheStatsResponse, SetCacheRequest,
};
use crate::services::ShardedCacheService;

/// Handler for `GET /api/v1/cache/{key}`.
///
/// # Arguments
/// * `path` - Key path parameter
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with `CacheItemResponse` or HTTP 404 Not Found.
pub async fn get_cache_key(
    path: web::Path<String>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> impl Responder {
    let key = path.into_inner();
    match cache.get(&key) {
        Some(item) => HttpResponse::Ok().json(item),
        None => HttpResponse::NotFound().json(json!({
            "status": 404,
            "error": "Key Not Found",
            "message": format!("Key '{}' does not exist in cache or has expired", key),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

/// Handler for `PUT /api/v1/cache/{key}`.
///
/// # Arguments
/// * `path` - Key path parameter
/// * `payload` - Inbound set cache JSON payload
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with shard index and confirmation.
pub async fn put_cache_key(
    path: web::Path<String>,
    payload: web::Json<SetCacheRequest>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> impl Responder {
    let key = path.into_inner();
    let data = payload.into_inner();
    let shard_id = cache.set(key.clone(), data.value, data.ttl_seconds);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "key": key,
        "shard_id": shard_id,
        "ttl_seconds": data.ttl_seconds,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handler for `DELETE /api/v1/cache/{key}`.
///
/// # Arguments
/// * `path` - Key path parameter
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with deletion status.
pub async fn delete_cache_key(
    path: web::Path<String>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> impl Responder {
    let key = path.into_inner();
    let deleted = cache.delete(&key);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "key": key,
        "deleted": deleted,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handler for `POST /api/v1/cache/batch/set`.
///
/// # Arguments
/// * `payload` - Batch set request JSON body
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with `BatchSetCacheResponse`.
pub async fn post_batch_set_cache(
    payload: web::Json<BatchSetCacheRequest>,
    cache: web::Data<Arc<ShardedCacheService>>,
) -> impl Responder {
    let start_time = Instant::now();
    let batch = payload.into_inner();
    let items_set = cache.batch_set(batch.items);
    let duration_us = start_time.elapsed().as_micros() as u64;

    HttpResponse::Ok().json(BatchSetCacheResponse {
        status: "success".to_string(),
        items_set,
        duration_us,
    })
}

/// Handler for `GET /api/v1/cache/stats`.
///
/// # Arguments
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with `CacheStatsResponse`.
pub async fn get_cache_stats(cache: web::Data<Arc<ShardedCacheService>>) -> impl Responder {
    let stats: CacheStatsResponse = cache.get_stats();
    HttpResponse::Ok().json(stats)
}

/// Handler for `POST /api/v1/cache/clear`.
///
/// # Arguments
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK confirmation.
pub async fn post_clear_cache(cache: web::Data<Arc<ShardedCacheService>>) -> impl Responder {
    cache.clear();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "All 64 cache shards have been cleared.",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Handler for `POST /api/v1/cache/purge-expired`.
///
/// # Arguments
/// * `cache` - Shared sharded cache service
///
/// # Returns
/// HTTP 200 OK with count of purged keys.
pub async fn post_purge_expired(cache: web::Data<Arc<ShardedCacheService>>) -> impl Responder {
    let purged = cache.purge_expired();
    HttpResponse::Ok().json(json!({
        "status": "success",
        "purged_keys_count": purged,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
