//! src/models/tracing.rs
//! W3C Trace Context and distributed tracing data models.
//! Connects to: src/middleware/tracing_middleware.rs, src/handlers/trace.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// W3C compliant trace context representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceContext {
    /// 16-byte (32 hex char) global trace identifier
    pub trace_id: String,
    /// 8-byte (16 hex char) current span identifier
    pub span_id: String,
    /// 8-byte (16 hex char) parent span identifier (if provided by caller)
    pub parent_span_id: Option<String>,
    /// 8-bit trace flags hex string (e.g. "01" = sampled)
    pub trace_flags: String,
    /// Optional W3C tracestate vendor metadata
    pub tracestate: Option<String>,
    /// Whether this request is sampled for distributed tracing
    pub is_sampled: bool,
}

impl TraceContext {
    /// Formats the W3C `traceparent` header string (`version-trace_id-span_id-trace_flags`).
    pub fn to_traceparent(&self) -> String {
        format!("00-{}-{}-{}", self.trace_id, self.span_id, self.trace_flags)
    }
}

/// Outbound response for the trace inspection endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInspectionResponse {
    /// Active W3C trace context for the current request
    pub context: TraceContext,
    /// Generated or propagated W3C traceparent header value
    pub traceparent: String,
    /// Inbound tracestate header value (if any)
    pub tracestate: Option<String>,
    /// Server processing timestamp in ISO 8601 UTC
    pub timestamp: String,
}
