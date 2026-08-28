//! src/middleware/mod.rs
//! Middleware module exports.
//! Connects to: src/middleware/latency_tracker.rs, src/main.rs
//! Created: 2026-08-27

pub mod latency_tracker;

pub use latency_tracker::LatencyTracker;
