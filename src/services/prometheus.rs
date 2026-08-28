//! src/services/prometheus.rs
//! Standard OpenMetrics / Prometheus 0.0.4 text exposition generator.
//! Connects to: src/services/metrics_service.rs, src/services/ring_buffer.rs, src/services/cache_service.rs
//! Created: 2026-08-28

use std::fmt::Write;

use crate::services::{MetricsService, RingBufferService, ShardedCacheService};

/// Renders system telemetry into standard Prometheus text/plain format (version 0.0.4).
///
/// # Arguments
/// * `metrics_service` - Global HTTP request telemetry service
/// * `ring_buffer` - In-memory event circular ring buffer
/// * `cache` - 64-way sharded cache service
///
/// # Returns
/// Formatted string compliant with Prometheus exposition format.
pub fn render_prometheus_metrics(
    metrics_service: &MetricsService,
    ring_buffer: &RingBufferService,
    cache: &ShardedCacheService,
) -> String {
    let snapshot = metrics_service.get_snapshot();
    let buffer_stats = ring_buffer.get_stats();
    let cache_stats = cache.get_stats();

    let mut out = String::with_capacity(4096);

    // 1. Process & Uptime
    writeln!(out, "# HELP process_uptime_seconds Total application uptime in seconds.").unwrap();
    writeln!(out, "# TYPE process_uptime_seconds counter").unwrap();
    writeln!(out, "process_uptime_seconds {}", snapshot.uptime_seconds).unwrap();

    // 2. HTTP Requests Counters
    writeln!(out, "# HELP http_requests_total Total HTTP requests processed by status family and route.").unwrap();
    writeln!(out, "# TYPE http_requests_total counter").unwrap();
    writeln!(out, "http_requests_total{{status_family=\"2xx\"}} {}", snapshot.status_2xx).unwrap();
    writeln!(out, "http_requests_total{{status_family=\"4xx\"}} {}", snapshot.status_4xx).unwrap();
    writeln!(out, "http_requests_total{{status_family=\"5xx\"}} {}", snapshot.status_5xx).unwrap();

    for (route, ep_data) in &snapshot.endpoints {
        writeln!(
            out,
            "http_route_requests_total{{route=\"{}\",status=\"success\"}} {}",
            route, ep_data.success_count
        )
        .unwrap();
        if ep_data.error_count > 0 {
            writeln!(
                out,
                "http_route_requests_total{{route=\"{}\",status=\"error\"}} {}",
                route, ep_data.error_count
            )
            .unwrap();
        }
    }

    // 3. Active in-flight requests
    writeln!(out, "# HELP http_requests_in_flight Current in-flight active HTTP requests.").unwrap();
    writeln!(out, "# TYPE http_requests_in_flight gauge").unwrap();
    writeln!(out, "http_requests_in_flight {}", snapshot.active_requests).unwrap();

    // 4. Bytes transmitted
    writeln!(out, "# HELP http_response_bytes_total Total response bytes sent.").unwrap();
    writeln!(out, "# TYPE http_response_bytes_total counter").unwrap();
    writeln!(out, "http_response_bytes_total {}", snapshot.total_bytes_sent).unwrap();

    // 5. Latency Statistics & Quantiles in Seconds
    let lat = &snapshot.latency_microseconds;
    writeln!(out, "# HELP http_request_duration_seconds HTTP request latency summary and percentiles in seconds.").unwrap();
    writeln!(out, "# TYPE http_request_duration_seconds summary").unwrap();
    writeln!(out, "http_request_duration_seconds{{quantile=\"0.5\"}} {:.6}", (lat.p50_us as f64) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds{{quantile=\"0.9\"}} {:.6}", (lat.p90_us as f64) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds{{quantile=\"0.95\"}} {:.6}", (lat.p95_us as f64) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds{{quantile=\"0.99\"}} {:.6}", (lat.p99_us as f64) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds{{quantile=\"0.999\"}} {:.6}", (lat.p999_us as f64) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds_sum {:.6}", (snapshot.total_requests as f64 * lat.mean_us) / 1_000_000.0).unwrap();
    writeln!(out, "http_request_duration_seconds_count {}", snapshot.total_requests).unwrap();

    // 6. Ring Buffer Telemetry
    writeln!(out, "# HELP ring_buffer_capacity Maximum slot capacity of circular ring buffer.").unwrap();
    writeln!(out, "# TYPE ring_buffer_capacity gauge").unwrap();
    writeln!(out, "ring_buffer_capacity {}", buffer_stats.capacity).unwrap();

    writeln!(out, "# HELP ring_buffer_occupancy Current active elements in circular ring buffer.").unwrap();
    writeln!(out, "# TYPE ring_buffer_occupancy gauge").unwrap();
    writeln!(out, "ring_buffer_occupancy {}", buffer_stats.current_occupancy).unwrap();

    writeln!(out, "# HELP ring_buffer_events_pushed_total Total events ingested into ring buffer.").unwrap();
    writeln!(out, "# TYPE ring_buffer_events_pushed_total counter").unwrap();
    writeln!(out, "ring_buffer_events_pushed_total {}", buffer_stats.total_pushed).unwrap();

    writeln!(out, "# HELP ring_buffer_events_dropped_total Total events overwritten/dropped on buffer overflow.").unwrap();
    writeln!(out, "# TYPE ring_buffer_events_dropped_total counter").unwrap();
    writeln!(out, "ring_buffer_events_dropped_total {}", buffer_stats.total_dropped).unwrap();

    // 7. Sharded Cache Telemetry
    writeln!(out, "# HELP cache_shards_total Number of cache partitions.").unwrap();
    writeln!(out, "# TYPE cache_shards_total gauge").unwrap();
    writeln!(out, "cache_shards_total {}", cache_stats.shard_count).unwrap();

    writeln!(out, "# HELP cache_keys_total Total active keys stored across all shards.").unwrap();
    writeln!(out, "# TYPE cache_keys_total gauge").unwrap();
    writeln!(out, "cache_keys_total {}", cache_stats.total_keys).unwrap();

    writeln!(out, "# HELP cache_hits_total Total successful cache hits.").unwrap();
    writeln!(out, "# TYPE cache_hits_total counter").unwrap();
    writeln!(out, "cache_hits_total {}", cache_stats.cache_hits).unwrap();

    writeln!(out, "# HELP cache_misses_total Total cache lookup misses.").unwrap();
    writeln!(out, "# TYPE cache_misses_total counter").unwrap();
    writeln!(out, "cache_misses_total {}", cache_stats.cache_misses).unwrap();

    writeln!(out, "# HELP cache_hit_ratio_percent Cache hit ratio percentage.").unwrap();
    writeln!(out, "# TYPE cache_hit_ratio_percent gauge").unwrap();
    writeln!(out, "cache_hit_ratio_percent {:.2}", cache_stats.hit_ratio_pct).unwrap();

    writeln!(out, "# HELP cache_expired_evictions_total Total keys evicted on TTL expiration.").unwrap();
    writeln!(out, "# TYPE cache_expired_evictions_total counter").unwrap();
    writeln!(out, "cache_expired_evictions_total {}", cache_stats.total_expired_evictions).unwrap();

    out
}
