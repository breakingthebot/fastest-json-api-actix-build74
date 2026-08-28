//! src/services/mod.rs
//! Services module exports.
//! Connects to: src/services/metrics_service.rs, src/main.rs
//! Created: 2026-08-27

pub mod metrics_service;

pub use metrics_service::MetricsService;
