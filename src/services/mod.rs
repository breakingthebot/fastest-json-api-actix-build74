//! src/services/mod.rs
//! Services module exports.
//! Connects to: src/services/metrics_service.rs, src/services/ring_buffer.rs, src/main.rs
//! Created: 2026-08-27

pub mod metrics_service;
pub mod ring_buffer;

pub use metrics_service::MetricsService;
pub use ring_buffer::RingBufferService;
