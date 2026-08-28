//! src/services/metrics_service.rs
//! High-performance lock-free atomic telemetry recorder and latency aggregator.
//! Connects to: src/models/metrics.rs, src/middleware/latency_tracker.rs, src/handlers/metrics.rs
//! Created: 2026-08-27

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::Instant;

use crate::models::{EndpointMetrics, LatencyDistribution, MetricsSnapshot};

const LATENCY_RESERVOIR_SIZE: usize = 10_000;

/// Shared application metrics service tracking live request telemetry.
pub struct MetricsService {
    /// Time when the server/service started
    start_time: Instant,
    /// Total processed HTTP requests counter
    total_requests: AtomicU64,
    /// In-flight active HTTP requests counter
    active_requests: AtomicUsize,
    /// Counter for 2xx Success responses
    status_2xx: AtomicU64,
    /// Counter for 4xx Client Error responses
    status_4xx: AtomicU64,
    /// Counter for 5xx Server Error responses
    status_5xx: AtomicU64,
    /// Total response body bytes transmitted
    total_bytes_sent: AtomicU64,
    /// Sum of all response latencies in microseconds for mean calculation
    total_latency_us: AtomicU64,
    /// Minimum recorded latency in microseconds
    min_latency_us: AtomicU64,
    /// Maximum recorded latency in microseconds
    max_latency_us: AtomicU64,
    /// Reservoir sampling of recent latencies for percentile calculations
    latency_samples: RwLock<Vec<u64>>,
    /// Next index for reservoir buffer insertion
    reservoir_index: AtomicUsize,
    /// Per-route request counters and statistics
    endpoint_data: RwLock<HashMap<String, EndpointTracker>>,
}

/// Internal accumulator for specific route telemetry.
#[derive(Default)]
struct EndpointTracker {
    total: u64,
    success: u64,
    errors: u64,
    total_latency_us: u64,
}

impl MetricsService {
    /// Initializes a new telemetry service instance with zeroed counters.
    ///
    /// # Returns
    /// An instantiated `MetricsService`.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            active_requests: AtomicUsize::new(0),
            status_2xx: AtomicU64::new(0),
            status_4xx: AtomicU64::new(0),
            status_5xx: AtomicU64::new(0),
            total_bytes_sent: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
            latency_samples: RwLock::new(Vec::with_capacity(LATENCY_RESERVOIR_SIZE)),
            reservoir_index: AtomicUsize::new(0),
            endpoint_data: RwLock::new(HashMap::new()),
        }
    }

    /// Increments the active in-flight request gauge.
    pub fn record_request_start(&self) {
        self.active_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Records completed request metrics including status code, byte length, route, and latency.
    ///
    /// # Arguments
    /// * `status_code` - HTTP response status code
    /// * `bytes_sent` - Size of response payload in bytes
    /// * `path` - Request route path
    /// * `duration_us` - Measured request latency in microseconds
    pub fn record_request_completion(
        &self,
        status_code: u16,
        bytes_sent: usize,
        path: &str,
        duration_us: u64,
    ) {
        self.active_requests.fetch_sub(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_sent.fetch_add(bytes_sent as u64, Ordering::Relaxed);
        self.total_latency_us.fetch_add(duration_us, Ordering::Relaxed);

        if status_code >= 200 && status_code < 300 {
            self.status_2xx.fetch_add(1, Ordering::Relaxed);
        } else if status_code >= 400 && status_code < 500 {
            self.status_4xx.fetch_add(1, Ordering::Relaxed);
        } else if status_code >= 500 {
            self.status_5xx.fetch_add(1, Ordering::Relaxed);
        }

        // Update Min / Max atomically using compare_exchange loops
        let mut current_min = self.min_latency_us.load(Ordering::Relaxed);
        while duration_us < current_min {
            match self.min_latency_us.compare_exchange_weak(
                current_min,
                duration_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }

        let mut current_max = self.max_latency_us.load(Ordering::Relaxed);
        while duration_us > current_max {
            match self.max_latency_us.compare_exchange_weak(
                current_max,
                duration_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }

        // Sample latency into reservoir buffer for percentile calculations
        let idx = self.reservoir_index.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut samples) = self.latency_samples.write() {
            if samples.len() < LATENCY_RESERVOIR_SIZE {
                samples.push(duration_us);
            } else {
                let slot = idx % LATENCY_RESERVOIR_SIZE;
                samples[slot] = duration_us;
            }
        }

        // Record per-endpoint telemetry
        if let Ok(mut endpoints) = self.endpoint_data.write() {
            let entry = endpoints.entry(path.to_string()).or_default();
            entry.total += 1;
            entry.total_latency_us += duration_us;
            if status_code < 400 {
                entry.success += 1;
            } else {
                entry.errors += 1;
            }
        }
    }

    /// Generates an immutable point-in-time snapshot of system performance metrics.
    ///
    /// # Returns
    /// A populated `MetricsSnapshot` struct.
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let uptime_seconds = self.start_time.elapsed().as_secs();
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let active_requests = self.active_requests.load(Ordering::Relaxed);
        let status_2xx = self.status_2xx.load(Ordering::Relaxed);
        let status_4xx = self.status_4xx.load(Ordering::Relaxed);
        let status_5xx = self.status_5xx.load(Ordering::Relaxed);
        let total_bytes_sent = self.total_bytes_sent.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);

        let average_rps = if uptime_seconds > 0 {
            (total_requests as f64) / (uptime_seconds as f64)
        } else {
            total_requests as f64
        };

        let mean_us = if total_requests > 0 {
            (total_latency as f64) / (total_requests as f64)
        } else {
            0.0
        };

        let min_val = self.min_latency_us.load(Ordering::Relaxed);
        let min_us = if min_val == u64::MAX { 0 } else { min_val };
        let max_us = self.max_latency_us.load(Ordering::Relaxed);

        let latency_dist = if let Ok(samples_guard) = self.latency_samples.read() {
            let mut sorted = samples_guard.clone();
            sorted.sort_unstable();
            let count = sorted.len();

            let calc_p = |pct: f64| -> u64 {
                if count == 0 {
                    0
                } else {
                    let idx = ((count as f64 * pct) / 100.0).round() as usize;
                    let clamped = idx.saturating_sub(1).min(count - 1);
                    sorted[clamped]
                }
            };

            LatencyDistribution {
                sample_count: count,
                min_us,
                mean_us,
                p50_us: calc_p(50.0),
                p90_us: calc_p(90.0),
                p95_us: calc_p(95.0),
                p99_us: calc_p(99.0),
                p999_us: calc_p(99.9),
                max_us,
            }
        } else {
            LatencyDistribution {
                sample_count: 0,
                min_us,
                mean_us,
                p50_us: 0,
                p90_us: 0,
                p95_us: 0,
                p99_us: 0,
                p999_us: 0,
                max_us,
            }
        };

        let mut endpoints_map = HashMap::new();
        if let Ok(endpoints_guard) = self.endpoint_data.read() {
            for (path, data) in endpoints_guard.iter() {
                let avg_latency = if data.total > 0 {
                    (data.total_latency_us as f64) / (data.total as f64)
                } else {
                    0.0
                };

                endpoints_map.insert(
                    path.clone(),
                    EndpointMetrics {
                        total_requests: data.total,
                        success_count: data.success,
                        error_count: data.errors,
                        avg_latency_us: avg_latency,
                    },
                );
            }
        }

        MetricsSnapshot {
            uptime_seconds,
            total_requests,
            active_requests,
            status_2xx,
            status_4xx,
            status_5xx,
            total_bytes_sent,
            average_rps,
            latency_microseconds: latency_dist,
            endpoints: endpoints_map,
        }
    }

    /// Resets all metric counters and buffers to initial zero state.
    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.status_2xx.store(0, Ordering::Relaxed);
        self.status_4xx.store(0, Ordering::Relaxed);
        self.status_5xx.store(0, Ordering::Relaxed);
        self.total_bytes_sent.store(0, Ordering::Relaxed);
        self.total_latency_us.store(0, Ordering::Relaxed);
        self.min_latency_us.store(u64::MAX, Ordering::Relaxed);
        self.max_latency_us.store(0, Ordering::Relaxed);
        self.reservoir_index.store(0, Ordering::Relaxed);

        if let Ok(mut samples) = self.latency_samples.write() {
            samples.clear();
        }

        if let Ok(mut endpoints) = self.endpoint_data.write() {
            endpoints.clear();
        }
    }
}

impl Default for MetricsService {
    fn default() -> Self {
        Self::new()
    }
}
