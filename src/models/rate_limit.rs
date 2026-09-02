//! src/models/rate_limit.rs
//! Token bucket rate limiting models and telemetry DTOs.
//! Connects to: src/services/rate_limiter.rs, src/middleware/rate_limit.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// Rate limit decision outcome returned by the token bucket engine.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimitDecision {
    /// Whether the request is permitted to proceed
    pub allowed: bool,
    /// Maximum burst bucket capacity
    pub limit: u64,
    /// Number of remaining tokens available in the client's bucket
    pub remaining: u64,
    /// Seconds until the client's token bucket is fully replenished
    pub reset_seconds: u64,
}

/// Outbound telemetry statistics for the Token Bucket rate limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatsResponse {
    /// Configured burst bucket capacity per client
    pub burst_capacity: u64,
    /// Configured token refill rate per second
    pub refill_rate_per_sec: f64,
    /// Total cumulative requests evaluated by rate limiter
    pub total_evaluated: u64,
    /// Total requests allowed through
    pub total_allowed: u64,
    /// Total requests blocked with HTTP 429
    pub total_rejected: u64,
    /// Total tracked active client IP/key buckets across all partitions
    pub active_client_buckets: usize,
    /// Rejection rate percentage
    pub rejection_ratio_pct: f64,
}

/// Error payload returned on HTTP 429 Too Many Requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitErrorResponse {
    /// HTTP status code (429)
    pub status: u16,
    /// Error summary ("Too Many Requests")
    pub error: String,
    /// Detailed rejection message
    pub message: String,
    /// Seconds until the client may retry
    pub retry_after_seconds: u64,
    /// Request URI instance
    pub instance: String,
    /// Timestamp in ISO 8601 UTC
    pub timestamp: String,
}
