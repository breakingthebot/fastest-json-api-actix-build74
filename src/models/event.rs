//! src/models/event.rs
//! Ingestion event models supporting zero-copy string borrowing and batch payloads.
//! Connects to: src/services/ring_buffer.rs, src/handlers/events.rs, src/models/mod.rs
//! Created: 2026-08-28

use serde::{Deserialize, Serialize};

/// Zero-copy deserialization schema borrowing string slices directly from the request byte buffer.
#[derive(Debug, Deserialize)]
pub struct ZeroCopyEvent<'a> {
    /// Event sequence or correlation ID borrowed from byte buffer
    pub event_id: &'a str,
    /// Event topic/category classification
    pub topic: &'a str,
    /// Originating service or device source tag
    pub source: &'a str,
    /// Event severity level ('info', 'warn', 'error', 'critical')
    pub severity: &'a str,
    /// Payload value in floating point or integer representation
    pub metric_value: f64,
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Owned event record stored in the in-memory circular ring buffer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestEvent {
    /// Unique event identifier
    pub id: u64,
    /// Event correlation string
    pub event_id: String,
    /// Topic or channel name
    pub topic: String,
    /// Originating emitter source
    pub source: String,
    /// Severity level
    pub severity: String,
    /// Numerical metric reading
    pub metric_value: f64,
    /// Timestamp when event occurred (ms)
    pub timestamp_ms: u64,
    /// Ingestion timestamp recorded by the API server (nanoseconds)
    pub ingested_at_nanos: u128,
}

impl IngestEvent {
    /// Converts a borrowed `ZeroCopyEvent` into an owned `IngestEvent`.
    ///
    /// # Arguments
    /// * `borrowed` - Borrowed event slice
    /// * `seq_id` - Sequential counter assigned by the ring buffer
    /// * `ingested_at_nanos` - Arrival timestamp in nanoseconds
    ///
    /// # Returns
    /// An instantiated `IngestEvent`.
    pub fn from_borrowed(borrowed: &ZeroCopyEvent<'_>, seq_id: u64, ingested_at_nanos: u128) -> Self {
        Self {
            id: seq_id,
            event_id: borrowed.event_id.to_string(),
            topic: borrowed.topic.to_string(),
            source: borrowed.source.to_string(),
            severity: borrowed.severity.to_string(),
            metric_value: borrowed.metric_value,
            timestamp_ms: borrowed.timestamp_ms,
            ingested_at_nanos,
        }
    }
}

/// Inbound batch event ingestion payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestRequest {
    /// Batch transmission ID
    pub batch_id: String,
    /// Sender client identifier
    pub client_id: String,
    /// Array of raw events to ingest into ring buffer
    pub events: Vec<BatchEventItem>,
}

/// Individual item within a batch ingestion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEventItem {
    /// Event identifier string
    pub event_id: String,
    /// Topic classification
    pub topic: String,
    /// Originating emitter
    pub source: String,
    /// Event severity
    pub severity: String,
    /// Numeric metric reading
    pub metric_value: f64,
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Outbound response for single or zero-copy event ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIngestResponse {
    /// Status message ('success', 'buffer_full')
    pub status: String,
    /// Assigned sequential ring buffer ID
    pub assigned_id: u64,
    /// Total events currently held in the ring buffer
    pub current_buffer_occupancy: usize,
    /// Processing latency in microseconds
    pub duration_us: u64,
}

/// Outbound response for batch event ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestResponse {
    /// Status message
    pub status: String,
    /// Echoed batch identifier
    pub batch_id: String,
    /// Number of events successfully committed to the ring buffer
    pub events_ingested: usize,
    /// Number of events dropped due to buffer overflow (if any)
    pub events_dropped: usize,
    /// Total current buffer occupancy
    pub buffer_occupancy: usize,
    /// Total batch processing latency in microseconds
    pub duration_us: u64,
}

/// Circular ring buffer state and occupancy telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStatsResponse {
    /// Maximum ring buffer capacity (elements)
    pub capacity: usize,
    /// Current number of active elements in buffer
    pub current_occupancy: usize,
    /// Total cumulative events pushed since startup
    pub total_pushed: u64,
    /// Total cumulative events overwritten/dropped on overflow
    pub total_dropped: u64,
    /// Current atomic write head sequence number
    pub write_head: usize,
    /// Current atomic read head sequence number
    pub read_head: usize,
    /// Estimated memory usage of the ring buffer in bytes
    pub memory_allocated_bytes: usize,
}

/// Query parameters for fetching recent events from the ring buffer.
#[derive(Debug, Deserialize)]
pub struct RecentEventsQuery {
    /// Maximum number of recent events to return (default: 20, max: 500)
    pub limit: Option<usize>,
    /// Filter events by topic name
    pub topic: Option<String>,
}

/// Outbound list of recent ring buffer events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEventsResponse {
    /// Number of events returned
    pub count: usize,
    /// Total current buffer occupancy
    pub total_occupancy: usize,
    /// Array of recent event items ordered newest first
    pub events: Vec<IngestEvent>,
}
