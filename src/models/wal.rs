//! src/models/wal.rs
//! Write-Ahead Log (WAL) data structures and telemetry models.
//! Connects to: src/services/wal_service.rs, src/handlers/wal.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// Comprehensive telemetry statistics for the Write-Ahead Log (WAL) persistence engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalStatsResponse {
    /// Active WAL file path on filesystem
    pub file_path: String,
    /// Current WAL file size on disk in bytes
    pub file_size_bytes: u64,
    /// Total cumulative events appended to WAL
    pub total_appends: u64,
    /// Total cumulative binary bytes written to WAL
    pub total_bytes_written: u64,
    /// Total valid events recovered and replayed on server startup
    pub recovered_on_boot: usize,
    /// Total corrupted or incomplete frames detected and skipped during recovery
    pub corrupted_frames_skipped: u64,
    /// Epoch timestamp in milliseconds when WAL was last synced to disk
    pub last_synced_ms: u64,
}

/// Response payload for forced disk synchronization (`POST /api/v1/wal/sync`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalSyncResponse {
    /// Status message ('success')
    pub status: String,
    /// Synchronization execution time in microseconds
    pub duration_us: u64,
    /// Total WAL file size flushed to durable storage in bytes
    pub file_size_bytes: u64,
    /// Timestamp in ISO 8601 UTC
    pub timestamp: String,
}

/// Response payload for WAL checkpoint and rotation (`POST /api/v1/wal/checkpoint`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalCheckpointResponse {
    /// Status message ('success')
    pub status: String,
    /// Descriptive checkpoint message
    pub message: String,
    /// WAL file size in bytes prior to truncation/checkpoint
    pub previous_size_bytes: u64,
    /// Timestamp in ISO 8601 UTC
    pub timestamp: String,
}
