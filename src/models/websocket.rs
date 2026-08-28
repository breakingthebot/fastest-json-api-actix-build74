//! src/models/websocket.rs
//! WebSocket real-time telemetry frames and client command models.
//! Connects to: src/services/websocket_broadcaster.rs, src/handlers/websocket.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// High-frequency telemetry snapshot broadcast to connected WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTelemetryFrame {
    /// Frame emission timestamp in ISO 8601 UTC
    pub timestamp: String,
    /// Process uptime in seconds
    pub uptime_seconds: u64,
    /// Total cumulative HTTP requests processed
    pub total_requests: u64,
    /// Number of concurrent requests in flight
    pub active_requests: usize,
    /// Current calculated requests per second (RPS)
    pub current_rps: f64,
    /// 50th percentile (median) internal server latency in microseconds
    pub p50_us: u64,
    /// 90th percentile latency in microseconds
    pub p90_us: u64,
    /// 99th percentile latency in microseconds
    pub p99_us: u64,
    /// Current element occupancy in the circular ring buffer
    pub ring_buffer_occupancy: usize,
    /// Total events pushed to ring buffer
    pub ring_buffer_total_pushed: u64,
    /// Total active keys stored in the 64-way sharded cache
    pub cache_total_keys: usize,
    /// Overall cache hit ratio percentage
    pub cache_hit_ratio_pct: f64,
}

/// Inbound command sent by WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsClientCommand {
    /// Command action: "ping", "get_snapshot", "reset_metrics", "drain_buffer"
    pub command: String,
    /// Optional parameter payload
    pub payload: Option<serde_json::Value>,
}

/// Outbound command acknowledgment sent back to WebSocket client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsCommandResponse {
    /// Status: "ok" or "error"
    pub status: String,
    /// Descriptive status message
    pub message: String,
    /// Optional data payload
    pub data: Option<serde_json::Value>,
    /// Server timestamp
    pub timestamp: String,
}
