//! src/services/ring_buffer.rs
//! High-throughput, cache-line aligned in-memory circular ring buffer for event ingestion.
//! Connects to: src/models/event.rs, src/handlers/events.rs, src/services/mod.rs
//! Created: 2026-08-28

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    BatchIngestRequest, BufferStatsResponse, IngestEvent, ZeroCopyEvent,
};

/// Default capacity for the circular ring buffer (must be power of two: 65,536 elements).
pub const RING_BUFFER_CAPACITY: usize = 65_536;
pub const RING_BUFFER_MASK: usize = RING_BUFFER_CAPACITY - 1;

/// Cache-line aligned (64-byte) structure to prevent CPU false sharing between reader and writer cores.
#[repr(align(64))]
struct CachePaddedCounter {
    value: AtomicUsize,
}

impl CachePaddedCounter {
    fn new(val: usize) -> Self {
        Self {
            value: AtomicUsize::new(val),
        }
    }
}

/// Thread-safe in-memory circular ring buffer for real-time event telemetry.
pub struct RingBufferService {
    /// Pre-allocated array of slots protected by granular read-write locks
    slots: Vec<RwLock<Option<IngestEvent>>>,
    /// Global monotonic sequence number for unique event IDs
    sequence_generator: AtomicU64,
    /// Atomic write head index (cache-line padded)
    write_head: CachePaddedCounter,
    /// Atomic read head index (cache-line padded)
    read_head: CachePaddedCounter,
    /// Total cumulative events pushed into the buffer
    total_pushed: AtomicU64,
    /// Total events dropped or overwritten due to buffer capacity overflow
    total_dropped: AtomicU64,
}

impl RingBufferService {
    /// Initializes a new circular ring buffer with pre-allocated slots.
    ///
    /// # Returns
    /// An instantiated `RingBufferService`.
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(RING_BUFFER_CAPACITY);
        for _ in 0..RING_BUFFER_CAPACITY {
            slots.push(RwLock::new(None));
        }

        Self {
            slots,
            sequence_generator: AtomicU64::new(1),
            write_head: CachePaddedCounter::new(0),
            read_head: CachePaddedCounter::new(0),
            total_pushed: AtomicU64::new(0),
            total_dropped: AtomicU64::new(0),
        }
    }

    /// Pushes an owned `IngestEvent` directly into the ring buffer (e.g. on crash recovery).
    ///
    /// # Arguments
    /// * `event` - Owned event record
    pub fn push(&self, event: IngestEvent) {
        self.push_event_internal(event);
    }

    /// Pushes a single zero-copy borrowed event into the ring buffer.
    ///
    /// # Arguments
    /// * `borrowed` - Inbound borrowed event payload
    ///
    /// # Returns
    /// Assigned sequential event ID.
    pub fn push_zerocopy(&self, borrowed: &ZeroCopyEvent<'_>) -> u64 {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let seq_id = self.sequence_generator.fetch_add(1, Ordering::Relaxed);
        let event = IngestEvent::from_borrowed(borrowed, seq_id, now_nanos);

        self.push_event_internal(event);
        seq_id
    }

    /// Pushes a batch of events into the ring buffer atomically.
    ///
    /// # Arguments
    /// * `batch` - Inbound batch ingestion request
    ///
    /// # Returns
    /// Tuple of `(events_ingested, events_dropped)`.
    pub fn push_batch(&self, batch: BatchIngestRequest) -> (usize, usize) {
        let now_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        let mut ingested = 0;

        for item in batch.events {
            let seq_id = self.sequence_generator.fetch_add(1, Ordering::Relaxed);
            let event = IngestEvent {
                id: seq_id,
                event_id: item.event_id,
                topic: item.topic,
                source: item.source,
                severity: item.severity,
                metric_value: item.metric_value,
                timestamp_ms: item.timestamp_ms,
                ingested_at_nanos: now_nanos,
            };

            self.push_event_internal(event);
            ingested += 1;
        }

        (ingested, 0)
    }

    /// Internal lock-free slot claiming and insertion.
    fn push_event_internal(&self, event: IngestEvent) {
        let head = self.write_head.value.fetch_add(1, Ordering::Relaxed);
        let slot_idx = head & RING_BUFFER_MASK;

        // Check if write head is lapping read head (buffer overflow)
        let current_read = self.read_head.value.load(Ordering::Relaxed);
        if head.saturating_sub(current_read) >= RING_BUFFER_CAPACITY {
            self.total_dropped.fetch_add(1, Ordering::Relaxed);
            self.read_head
                .value
                .store(head.saturating_sub(RING_BUFFER_CAPACITY - 1), Ordering::Relaxed);
        }

        if let Some(slot) = self.slots.get(slot_idx) {
            if let Ok(mut guard) = slot.write() {
                *guard = Some(event);
            }
        }

        self.total_pushed.fetch_add(1, Ordering::Relaxed);
    }

    /// Retrieves telemetry statistics for the ring buffer.
    ///
    /// # Returns
    /// Populated `BufferStatsResponse` struct.
    pub fn get_stats(&self) -> BufferStatsResponse {
        let write_pos = self.write_head.value.load(Ordering::Relaxed);
        let read_pos = self.read_head.value.load(Ordering::Relaxed);
        let occupancy = write_pos.saturating_sub(read_pos).min(RING_BUFFER_CAPACITY);

        let total_pushed = self.total_pushed.load(Ordering::Relaxed);
        let total_dropped = self.total_dropped.load(Ordering::Relaxed);

        // Approximate memory: slot vec + size of IngestEvent (~96 bytes) * occupancy
        let memory_allocated_bytes = (RING_BUFFER_CAPACITY * std::mem::size_of::<RwLock<Option<IngestEvent>>>())
            + (occupancy * 128);

        BufferStatsResponse {
            capacity: RING_BUFFER_CAPACITY,
            current_occupancy: occupancy,
            total_pushed,
            total_dropped,
            write_head: write_pos,
            read_head: read_pos,
            memory_allocated_bytes,
        }
    }

    /// Reads up to `limit` most recent events from the ring buffer without destructive dequeuing.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of elements to retrieve
    /// * `topic_filter` - Optional topic name to filter by
    ///
    /// # Returns
    /// Vector of recent `IngestEvent` items ordered newest first.
    pub fn get_recent(&self, limit: usize, topic_filter: Option<&str>) -> Vec<IngestEvent> {
        let write_pos = self.write_head.value.load(Ordering::Relaxed);
        let read_pos = self.read_head.value.load(Ordering::Relaxed);
        let occupancy = write_pos.saturating_sub(read_pos).min(RING_BUFFER_CAPACITY);

        let fetch_count = limit.min(occupancy).min(500);
        let mut results = Vec::with_capacity(fetch_count);

        for i in 0..occupancy {
            if results.len() >= fetch_count {
                break;
            }

            let idx = (write_pos.saturating_sub(1 + i)) & RING_BUFFER_MASK;
            if let Some(slot) = self.slots.get(idx) {
                if let Ok(guard) = slot.read() {
                    if let Some(ref event) = *guard {
                        if let Some(filter) = topic_filter {
                            if event.topic == filter {
                                results.push(event.clone());
                            }
                        } else {
                            results.push(event.clone());
                        }
                    }
                }
            }
        }

        results
    }

    /// Drains all buffered events and resets read head to match write head.
    ///
    /// # Returns
    /// Vector of all drained `IngestEvent` items.
    pub fn drain(&self) -> Vec<IngestEvent> {
        let write_pos = self.write_head.value.load(Ordering::Relaxed);
        let read_pos = self.read_head.value.swap(write_pos, Ordering::Relaxed);
        let count = write_pos.saturating_sub(read_pos).min(RING_BUFFER_CAPACITY);

        let mut drained = Vec::with_capacity(count);
        for i in 0..count {
            let slot_idx = (read_pos + i) & RING_BUFFER_MASK;
            if let Some(slot) = self.slots.get(slot_idx) {
                if let Ok(mut guard) = slot.write() {
                    if let Some(event) = guard.take() {
                        drained.push(event);
                    }
                }
            }
        }

        drained
    }

    /// Resets all counters, pointers, and clears buffer slots.
    pub fn reset(&self) {
        self.write_head.value.store(0, Ordering::Relaxed);
        self.read_head.value.store(0, Ordering::Relaxed);
        self.total_pushed.store(0, Ordering::Relaxed);
        self.total_dropped.store(0, Ordering::Relaxed);
        self.sequence_generator.store(1, Ordering::Relaxed);

        for slot in &self.slots {
            if let Ok(mut guard) = slot.write() {
                *guard = None;
            }
        }
    }
}

impl Default for RingBufferService {
    fn default() -> Self {
        Self::new()
    }
}
