//! src/models/cache.rs
//! Data models for the 64-way sharded in-memory cache engine and telemetry.
//! Connects to: src/services/cache_service.rs, src/handlers/cache.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// Inbound request payload for setting a cache value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCacheRequest {
    /// Arbitrary JSON value to store in cache
    pub value: serde_json::Value,
    /// Optional time-to-live in seconds (None = persistent until evicted or restarted)
    pub ttl_seconds: Option<u64>,
}

/// Outbound response payload when retrieving a cached key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheItemResponse {
    /// Cached key name
    pub key: String,
    /// Stored JSON value
    pub value: serde_json::Value,
    /// Shard index (0..63) where this key is stored
    pub shard_id: usize,
    /// Total cache hits for this key
    pub hits: u64,
    /// Remaining time-to-live in milliseconds (None if no TTL configured)
    pub ttl_remaining_ms: Option<i64>,
    /// Creation timestamp in milliseconds
    pub created_at_ms: u64,
}

/// Inbound request for batch setting multiple cache keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSetCacheRequest {
    /// Array of key-value pairs with optional per-item TTL
    pub items: Vec<BatchCacheItem>,
}

/// Individual item in batch cache set request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCacheItem {
    /// Unique cache key
    pub key: String,
    /// JSON value to store
    pub value: serde_json::Value,
    /// Optional item TTL in seconds
    pub ttl_seconds: Option<u64>,
}

/// Outbound response for batch cache setting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSetCacheResponse {
    /// Status message ('success')
    pub status: String,
    /// Number of items successfully committed to cache shards
    pub items_set: usize,
    /// Execution duration in microseconds
    pub duration_us: u64,
}

/// Comprehensive telemetry statistics for the 64-way sharded cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatsResponse {
    /// Total number of shards configured (64)
    pub shard_count: usize,
    /// Total active non-expired keys across all shards
    pub total_keys: usize,
    /// Total cumulative GET requests
    pub total_gets: u64,
    /// Total successful cache hits
    pub cache_hits: u64,
    /// Total cache misses
    pub cache_misses: u64,
    /// Hit ratio percentage (0.0 to 100.0%)
    pub hit_ratio_pct: f64,
    /// Total cumulative SET operations
    pub total_sets: u64,
    /// Total cumulative DELETE operations
    pub total_deletes: u64,
    /// Total keys evicted due to expired TTL
    pub total_expired_evictions: u64,
    /// Estimated memory usage in bytes across all shards
    pub estimated_memory_bytes: usize,
    /// Breakdown of key counts per shard
    pub shard_distribution: Vec<ShardStats>,
}

/// Per-shard telemetry summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardStats {
    /// Shard identifier (0..63)
    pub shard_id: usize,
    /// Number of active keys in this shard
    pub key_count: usize,
}
