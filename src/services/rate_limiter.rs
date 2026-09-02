//! src/services/rate_limiter.rs
//! High-throughput 64-way sharded Token Bucket rate limiting engine.
//! Connects to: src/models/rate_limit.rs, src/middleware/rate_limit.rs, src/services/mod.rs
//! Created: 2026-08-28

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{RateLimitDecision, RateLimitStatsResponse};

const NUM_BUCKET_SHARDS: usize = 64;
const BUCKET_SHARD_MASK: usize = NUM_BUCKET_SHARDS - 1;

/// Individual client token bucket state.
#[derive(Debug, Clone)]
struct ClientBucket {
    tokens: f64,
    last_refill_ms: u64,
}

/// Sharded partition of client buckets protected by an isolated Read-Write lock.
struct RateLimitShard {
    buckets: RwLock<HashMap<String, ClientBucket>>,
}

/// Thread-safe 64-way partitioned token bucket rate limiter eliminating lock contention.
pub struct RateLimiterService {
    shards: Vec<RateLimitShard>,
    burst_capacity: u64,
    refill_rate_per_sec: f64,
    total_evaluated: AtomicU64,
    total_allowed: AtomicU64,
    total_rejected: AtomicU64,
}

impl RateLimiterService {
    /// Initializes a new rate limiter with specified burst capacity and refill rate.
    ///
    /// # Arguments
    /// * `burst_capacity` - Maximum burst token allowance per client
    /// * `refill_rate_per_sec` - Tokens replenished per second
    ///
    /// # Returns
    /// An instantiated `RateLimiterService`.
    pub fn new(burst_capacity: u64, refill_rate_per_sec: f64) -> Self {
        let mut shards = Vec::with_capacity(NUM_BUCKET_SHARDS);
        for _ in 0..NUM_BUCKET_SHARDS {
            shards.push(RateLimitShard {
                buckets: RwLock::new(HashMap::with_capacity(64)),
            });
        }

        Self {
            shards,
            burst_capacity,
            refill_rate_per_sec,
            total_evaluated: AtomicU64::new(0),
            total_allowed: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
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
        (hash ^ (hash >> 16)) as usize & BUCKET_SHARD_MASK
    }

    /// Current epoch timestamp in milliseconds.
    #[inline]
    fn current_epoch_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Evaluates whether a request from the given client key is permitted under the Token Bucket rules.
    ///
    /// # Arguments
    /// * `client_key` - Unique client identifier (e.g. IP address or API key)
    ///
    /// # Returns
    /// `RateLimitDecision` struct with permission flag, remaining allowance, and reset seconds.
    pub fn try_acquire(&self, client_key: &str) -> RateLimitDecision {
        self.total_evaluated.fetch_add(1, Ordering::Relaxed);
        let shard_id = self.get_shard_index(client_key);
        let now_ms = Self::current_epoch_ms();
        let cap_f64 = self.burst_capacity as f64;

        let shard = &self.shards[shard_id];

        if let Ok(mut guard) = shard.buckets.write() {
            let bucket = guard
                .entry(client_key.to_string())
                .or_insert_with(|| ClientBucket {
                    tokens: cap_f64,
                    last_refill_ms: now_ms,
                });

            // Calculate token replenishment since last access
            let elapsed_ms = now_ms.saturating_sub(bucket.last_refill_ms);
            let elapsed_secs = (elapsed_ms as f64) / 1000.0;
            let replenished = elapsed_secs * self.refill_rate_per_sec;

            bucket.tokens = (bucket.tokens + replenished).min(cap_f64);
            bucket.last_refill_ms = now_ms;

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                let remaining = bucket.tokens.floor() as u64;
                let reset_seconds = ((cap_f64 - bucket.tokens) / self.refill_rate_per_sec).ceil() as u64;

                self.total_allowed.fetch_add(1, Ordering::Relaxed);
                RateLimitDecision {
                    allowed: true,
                    limit: self.burst_capacity,
                    remaining,
                    reset_seconds: reset_seconds.max(1),
                }
            } else {
                let needed = 1.0 - bucket.tokens;
                let reset_seconds = (needed / self.refill_rate_per_sec).ceil() as u64;

                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                RateLimitDecision {
                    allowed: false,
                    limit: self.burst_capacity,
                    remaining: 0,
                    reset_seconds: reset_seconds.max(1),
                }
            }
        } else {
            // Fallback allow on lock poisoning
            self.total_allowed.fetch_add(1, Ordering::Relaxed);
            RateLimitDecision {
                allowed: true,
                limit: self.burst_capacity,
                remaining: self.burst_capacity,
                reset_seconds: 1,
            }
        }
    }

    /// Resets all client buckets and counters.
    pub fn reset(&self) {
        for shard in &self.shards {
            if let Ok(mut guard) = shard.buckets.write() {
                guard.clear();
            }
        }
        self.total_evaluated.store(0, Ordering::Relaxed);
        self.total_allowed.store(0, Ordering::Relaxed);
        self.total_rejected.store(0, Ordering::Relaxed);
    }

    /// Compiles telemetry statistics for the Token Bucket rate limiter.
    pub fn get_stats(&self) -> RateLimitStatsResponse {
        let total_evaluated = self.total_evaluated.load(Ordering::Relaxed);
        let total_allowed = self.total_allowed.load(Ordering::Relaxed);
        let total_rejected = self.total_rejected.load(Ordering::Relaxed);

        let rejection_ratio_pct = if total_evaluated > 0 {
            ((total_rejected as f64) / (total_evaluated as f64)) * 100.0
        } else {
            0.0
        };

        let mut active_client_buckets = 0;
        for shard in &self.shards {
            if let Ok(guard) = shard.buckets.read() {
                active_client_buckets += guard.len();
            }
        }

        RateLimitStatsResponse {
            burst_capacity: self.burst_capacity,
            refill_rate_per_sec: self.refill_rate_per_sec,
            total_evaluated,
            total_allowed,
            total_rejected,
            active_client_buckets,
            rejection_ratio_pct,
        }
    }
}

impl Default for RateLimiterService {
    fn default() -> Self {
        // Default: 500 burst capacity, 200 tokens/sec refill
        Self::new(500, 200.0)
    }
}
