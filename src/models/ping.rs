//! src/models/ping.rs
//! Ultra-low-overhead ping response schema.
//! Connects to: src/handlers/ping.rs, src/models/mod.rs
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};

/// Lightweight ping response designed for sub-millisecond heartbeat verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PingResponse {
    /// Status confirmation constant ("pong")
    pub message: String,
    /// High-resolution epoch timestamp in milliseconds
    pub timestamp_ms: i64,
    /// Nanoseconds component of current system clock
    pub unix_nanos: u128,
}
