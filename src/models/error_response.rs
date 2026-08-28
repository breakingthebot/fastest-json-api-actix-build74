//! src/models/error_response.rs
//! Standardized RFC 7807 problem details error response schema.
//! Connects to: src/models/mod.rs, application error responders
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};

/// RFC 7807 compliant problem details error response structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiErrorResponse {
    /// HTTP status code
    pub status: u16,
    /// Short human-readable summary of the error type
    pub error: String,
    /// Detailed explanation specific to this occurrence of the problem
    pub message: String,
    /// Timestamp when error occurred in UTC ISO 8601
    pub timestamp: String,
    /// Request path that caused the failure
    pub path: String,
}

impl ApiErrorResponse {
    /// Creates a new `ApiErrorResponse` instance.
    ///
    /// # Arguments
    /// * `status` - HTTP status code
    /// * `error` - Error category string
    /// * `message` - Detailed context
    /// * `path` - Request URI path
    ///
    /// # Returns
    /// Formatted error response object.
    pub fn new(status: u16, error: impl Into<String>, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            status,
            error: error.into(),
            message: message.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: path.into(),
        }
    }
}
