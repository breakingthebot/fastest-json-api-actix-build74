//! src/services/cache_service.rs
//! High-performance 64-way sharded in-memory cache engine with sub-microsecond TTL expiration.
//! Connects to: src/models/cache.rs, src/handlers/cache.rs, src/services/mod.rs
//! Created: 2026-08-28

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    BatchCacheItem, CacheItemResponse, CacheStatsResponse, ShardStats,
};

/// Number of independent cache partitions (power of two: 64 shards).
pub const NUM_SHARDS: usize = 64;
pub const SHARD_MASK: usize = NUM_SHARDS - 1;

/// Individual cached value record.
pub struct CacheEntry {
    /// Stored JSON payload
    pub value: serde_json::Value,
    /// Epoch timestamp when inserted (ms)
    pub created_at_ms: u64,
    /// Epoch timestamp when key expires (ms), if configured
    pub expires_at_ms: Option<u64>,
    /// Monotonic hit counter
    pub hits: AtomicU64,
    /// Estimated byte size of JSON payload
    pub byte_size: usize,
}

/// A single cache partition with isolated Read-Write lock.
struct CacheShard {
    entries: RwLock<HashMap<String, CacheEntry>>,
}

/// Thread-safe, 64-way partitioned cache service eliminating global lock contention.
pub struct ShardedCacheService {
    shards: Vec<CacheShard>,
    total_gets: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    total_sets: AtomicU64,
    total_deletes: AtomicU64,
    total_expired_evictions: AtomicU64,
}

impl ShardedCacheService {
    /// Initializes a new 64-way sharded cache service with pre-allocated lock partitions.
    ///
    /// # Returns
    /// An instantiated `ShardedCacheService`.
    pub fn new() -> Self {
        let mut shards = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            shards.push(CacheShard {
                entries: RwLock::new(HashMap::with_capacity(128)),
            });
        }

        Self {
            shards,
            total_gets: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_sets: AtomicU64::new(0),
            total_deletes: AtomicU64::new(0),
            total_expired_evictions: AtomicU64::new(0),
        }
    }

    /// Fast non-cryptographic FNV-1a hash to determine target shard index (0..63).
    #[inline]
    fn get_shard_index(&self, key: &str) -> usize {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in key.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        (hash ^ (hash >> 16)) as usize & SHARD_MASK
    }

    /// Current epoch timestamp in milliseconds.
    #[inline]
    fn current_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Retrieves an item from the cache, verifying TTL expiration and recording hit metrics.
    ///
    /// # Arguments
    /// * `key` - Cache key lookup string
    ///
    /// # Returns
    /// `Some(CacheItemResponse)` if found and unexpired, `None` if missing or expired.
    pub fn get(&self, key: &str) -> Option<CacheItemResponse> {
        self.total_gets.fetch_add(1, Ordering::Relaxed);
        let shard_id = self.get_shard_index(key);
        let now_ms = Self::current_epoch_ms();

        let shard = &self.shards[shard_id];

        // Fast path: acquire read lock
        if let Ok(guard) = shard.entries.read() {
            if let Some(entry) = guard.get(key) {
                // Check if expired
                if let Some(expires_at) = entry.expires_at_ms {
                    if now_ms >= expires_at {
                        drop(guard);
                        // Expired: drop read lock and purge under write lock
                        if let Ok(mut write_guard) = shard.entries.write() {
                            if write_guard.remove(key).is_some() {
                                self.total_expired_evictions.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        self.cache_misses.fetch_add(1, Ordering::Relaxed);
                        return None;
                    }
                }

                // Cache hit: increment hit counter and return cloned response
                let hits = entry.hits.fetch_add(1, Ordering::Relaxed) + 1;
                self.cache_hits.fetch_add(1, Ordering::Relaxed);

                let ttl_remaining_ms = entry
                    .expires_at_ms
                    .map(|exp| (exp as i64) - (now_ms as i64));

                return Some(CacheItemResponse {
                    key: key.to_string(),
                    value: entry.value.clone(),
                    shard_id,
                    hits,
                    ttl_remaining_ms,
                    created_at_ms: entry.created_at_ms,
                });
            }
        }

        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Sets or updates a cache key with optional TTL in seconds.
    ///
    /// # Arguments
    /// * `key` - Unique key string
    /// * `value` - JSON payload
    /// * `ttl_seconds` - Optional TTL duration in seconds
    ///
    /// # Returns
    /// Shard index where key was committed.
    pub fn set(&self, key: String, value: serde_json::Value, ttl_seconds: Option<u64>) -> usize {
        self.total_sets.fetch_add(1, Ordering::Relaxed);
        let shard_id = self.get_shard_index(&key);
        let now_ms = Self::current_epoch_ms();

        let expires_at_ms = ttl_seconds.map(|secs| now_ms + (secs * 1000));
        let byte_size = serde_json::to_vec(&value).map(|v| v.len()).unwrap_or(32);

        let entry = CacheEntry {
            value,
            created_at_ms: now_ms,
            expires_at_ms,
            hits: AtomicU64::new(0),
            byte_size,
        };

        if let Ok(mut guard) = self.shards[shard_id].entries.write() {
            guard.insert(key, entry);
        }

        shard_id
    }

    /// Sets multiple key-value items across respective cache shards in batch.
    ///
    /// # Arguments
    /// * `items` - Vector of batch cache items
    ///
    /// # Returns
    /// Total items successfully committed.
    pub fn batch_set(&self, items: Vec<BatchCacheItem>) -> usize {
        let count = items.len();
        for item in items {
            self.set(item.key, item.value, item.ttl_seconds);
        }
        count
    }

    /// Deletes a key from the cache.
    ///
    /// # Arguments
    /// * `key` - Key name to remove
    ///
    /// # Returns
    /// `true` if key was present and removed, `false` otherwise.
    pub fn delete(&self, key: &str) -> bool {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);
        let shard_id = self.get_shard_index(key);

        if let Ok(mut guard) = self.shards[shard_id].entries.write() {
            guard.remove(key).is_some()
        } else {
            false
        }
    }

    /// Purges all expired keys across all 64 shards.
    ///
    /// # Returns
    /// Number of expired keys evicted.
    pub fn purge_expired(&self) -> usize {
        let now_ms = Self::current_epoch_ms();
        let mut evicted = 0;

        for shard in &self.shards {
            if let Ok(mut guard) = shard.entries.write() {
                let initial_len = guard.len();
                guard.retain(|_, entry| {
                    if let Some(exp) = entry.expires_at_ms {
                        now_ms < exp
                    } else {
                        true
                    }
                });
                let diff = initial_len.saturating_sub(guard.len());
                evicted += diff;
            }
        }

        self.total_expired_evictions
            .fetch_add(evicted as u64, Ordering::Relaxed);
        evicted
    }

    /// Clears all keys across all 64 shards.
    pub fn clear(&self) {
        for shard in &self.shards {
            if let Ok(mut guard) = shard.entries.write() {
                guard.clear();
            }
        }
    }

    /// Compiles telemetry statistics across all shards.
    ///
    /// # Returns
    /// Populated `CacheStatsResponse` struct.
    pub fn get_stats(&self) -> CacheStatsResponse {
        let total_gets = self.total_gets.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let total_sets = self.total_sets.load(Ordering::Relaxed);
        let total_deletes = self.total_deletes.load(Ordering::Relaxed);
        let total_expired_evictions = self.total_expired_evictions.load(Ordering::Relaxed);

        let hit_ratio_pct = if total_gets > 0 {
            ((cache_hits as f64) / (total_gets as f64)) * 100.0
        } else {
            0.0
        };

        let mut total_keys = 0;
        let mut estimated_memory_bytes = NUM_SHARDS * std::mem::size_of::<CacheShard>();
        let mut shard_distribution = Vec::with_capacity(NUM_SHARDS);

        for (shard_id, shard) in self.shards.iter().enumerate() {
            if let Ok(guard) = shard.entries.read() {
                let count = guard.len();
                total_keys += count;
                for entry in guard.values() {
                    estimated_memory_bytes += entry.byte_size + 64;
                }
                shard_distribution.push(ShardStats { shard_id, key_count: count });
            } else {
                shard_distribution.push(ShardStats { shard_id, key_count: 0 });
            }
        }

        CacheStatsResponse {
            shard_count: NUM_SHARDS,
            total_keys,
            total_gets,
            cache_hits,
            cache_misses,
            hit_ratio_pct,
            total_sets,
            total_deletes,
            total_expired_evictions,
            estimated_memory_bytes,
            shard_distribution,
        }
    }
}

impl Default for ShardedCacheService {
    fn default() -> Self {
        Self::new()
    }
}
