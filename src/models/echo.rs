//! src/models/echo.rs
//! Payload models for JSON round-trip echo and serialization testing.
//! Connects to: src/handlers/echo.rs, src/models/mod.rs
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Inbound JSON echo request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoRequest {
    /// Message string
    pub message: String,
    /// Arbitrary structured metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Optional numeric test value
    #[serde(default)]
    pub count: Option<i64>,
    /// Optional flag
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Optional nested string array
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Outbound JSON echo response payload with processing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoResponse {
    /// Echoed back request payload
    pub received: EchoRequest,
    /// Calculated byte size of the parsed JSON payload
    pub payload_bytes: usize,
    /// Number of elements in tags array
    pub tag_count: usize,
    /// Server processing timestamp in ISO 8601 UTC
    pub processed_at: String,
    /// Processing duration in microseconds
    pub server_processing_us: u64,
}
