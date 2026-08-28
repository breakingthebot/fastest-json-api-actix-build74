//! src/models/metrics.rs
//! Telemetry and latency distribution statistical models.
//! Connects to: src/services/metrics_service.rs, src/handlers/metrics.rs, src/models/mod.rs
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Comprehensive server telemetry snapshot including request rates and latency distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Server uptime in seconds
    pub uptime_seconds: u64,
    /// Total HTTP requests processed since startup or last reset
    pub total_requests: u64,
    /// Currently in-flight active HTTP requests
    pub active_requests: usize,
    /// Total successful responses (2xx status codes)
    pub status_2xx: u64,
    /// Total client error responses (4xx status codes)
    pub status_4xx: u64,
    /// Total server error responses (5xx status codes)
    pub status_5xx: u64,
    /// Total bytes served in response bodies
    pub total_bytes_sent: u64,
    /// Calculated requests-per-second (RPS) over uptime
    pub average_rps: f64,
    /// High-resolution latency statistics in microseconds (μs)
    pub latency_microseconds: LatencyDistribution,
    /// Breakdown of request counts by endpoint route
    pub endpoints: HashMap<String, EndpointMetrics>,
}

/// Latency statistics and percentiles measured in microseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    /// Total latency samples collected
    pub sample_count: usize,
    /// Minimum recorded latency in microseconds
    pub min_us: u64,
    /// Arithmetic mean latency in microseconds
    pub mean_us: f64,
    /// 50th percentile (median) latency in microseconds
    pub p50_us: u64,
    /// 90th percentile latency in microseconds
    pub p90_us: u64,
    /// 95th percentile latency in microseconds
    pub p95_us: u64,
    /// 99th percentile latency in microseconds
    pub p99_us: u64,
    /// 99.9th percentile latency in microseconds
    pub p999_us: u64,
    /// Maximum recorded latency in microseconds
    pub max_us: u64,
}

/// Endpoint-specific metrics counter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndpointMetrics {
    /// Total requests hitting this specific route
    pub total_requests: u64,
    /// Total successful responses (2xx) for this route
    pub success_count: u64,
    /// Total error responses (4xx/5xx) for this route
    pub error_count: u64,
    /// Average latency in microseconds for this route
    pub avg_latency_us: f64,
}
