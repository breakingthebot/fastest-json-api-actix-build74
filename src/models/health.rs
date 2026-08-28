//! src/models/health.rs
//! Health check and system status response models.
//! Connects to: src/handlers/health.rs, src/models/mod.rs
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};

/// System health check response schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    /// Overall service status ('healthy', 'degraded', 'unhealthy')
    pub status: String,
    /// Semantic API version
    pub version: String,
    /// Service identifier
    pub service: String,
    /// Current runtime environment
    pub environment: String,
    /// Server uptime in elapsed seconds
    pub uptime_seconds: u64,
    /// ISO 8601 UTC server timestamp
    pub timestamp: String,
    /// Number of configured worker threads
    pub worker_threads: usize,
    /// Architecture and OS metadata
    pub system: SystemMetadata,
}

/// Server architecture and system runtime metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemMetadata {
    /// Target operating system (e.g. windows, linux, macos)
    pub os: String,
    /// Target CPU architecture (e.g. x86_64, aarch64)
    pub arch: String,
    /// Number of available logical CPU cores
    pub num_cpus: usize,
}
