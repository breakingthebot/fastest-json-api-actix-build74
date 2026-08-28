//! src/services/websocket_broadcaster.rs
//! High-frequency real-time WebSocket telemetry broadcaster.
//! Connects to: src/models/websocket.rs, src/services/metrics_service.rs, src/services/ring_buffer.rs, src/services/cache_service.rs
//! Created: 2026-08-28

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::models::LiveTelemetryFrame;
use crate::services::{MetricsService, RingBufferService, ShardedCacheService};

/// Maximum broadcast channel buffer capacity for connected WebSocket clients.
const BROADCAST_CAPACITY: usize = 128;

/// Central broadcast service managing high-frequency live metric streaming to WebSocket clients.
pub struct WebSocketBroadcaster {
    sender: broadcast::Sender<LiveTelemetryFrame>,
    prev_requests: AtomicU64,
}

impl WebSocketBroadcaster {
    /// Initializes a new WebSocket broadcaster and starts a background 100ms emission loop.
    ///
    /// # Arguments
    /// * `metrics_service` - Global metrics service
    /// * `ring_buffer` - Circular ring buffer service
    /// * `cache` - 64-way sharded cache service
    ///
    /// # Returns
    /// An instantiated `Arc<WebSocketBroadcaster>`.
    pub fn new(
        metrics_service: Arc<MetricsService>,
        ring_buffer: Arc<RingBufferService>,
        cache: Arc<ShardedCacheService>,
    ) -> Arc<Self> {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        let broadcaster = Arc::new(Self {
            sender,
            prev_requests: AtomicU64::new(0),
        });

        let broadcaster_clone = Arc::clone(&broadcaster);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut last_tick = Instant::now();

            loop {
                interval.tick().await;
                let now = Instant::now();
                let delta_secs = now.duration_since(last_tick).as_secs_f64().max(0.001);
                last_tick = now;

                let snapshot = metrics_service.get_snapshot();
                let buffer_stats = ring_buffer.get_stats();
                let cache_stats = cache.get_stats();

                let prev = broadcaster_clone.prev_requests.swap(snapshot.total_requests, Ordering::Relaxed);
                let delta_requests = snapshot.total_requests.saturating_sub(prev);
                let current_rps = (delta_requests as f64) / delta_secs;

                let frame = LiveTelemetryFrame {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    uptime_seconds: snapshot.uptime_seconds,
                    total_requests: snapshot.total_requests,
                    active_requests: snapshot.active_requests,
                    current_rps,
                    p50_us: snapshot.latency_microseconds.p50_us,
                    p90_us: snapshot.latency_microseconds.p90_us,
                    p99_us: snapshot.latency_microseconds.p99_us,
                    ring_buffer_occupancy: buffer_stats.current_occupancy,
                    ring_buffer_total_pushed: buffer_stats.total_pushed,
                    cache_total_keys: cache_stats.total_keys,
                    cache_hit_ratio_pct: cache_stats.hit_ratio_pct,
                };

                // Non-blocking broadcast; receivers lag silently if slow
                let _ = broadcaster_clone.sender.send(frame);
            }
        });

        broadcaster
    }

    /// Subscribes a new WebSocket connection to receive real-time telemetry frames.
    pub fn subscribe(&self) -> broadcast::Receiver<LiveTelemetryFrame> {
        self.sender.subscribe()
    }

    /// Compiles an immediate telemetry snapshot frame for instant delivery.
    pub fn build_current_frame(
        &self,
        metrics_service: &MetricsService,
        ring_buffer: &RingBufferService,
        cache: &ShardedCacheService,
    ) -> LiveTelemetryFrame {
        let snapshot = metrics_service.get_snapshot();
        let buffer_stats = ring_buffer.get_stats();
        let cache_stats = cache.get_stats();

        LiveTelemetryFrame {
            timestamp: chrono::Utc::now().to_rfc3339(),
            uptime_seconds: snapshot.uptime_seconds,
            total_requests: snapshot.total_requests,
            active_requests: snapshot.active_requests,
            current_rps: 0.0,
            p50_us: snapshot.latency_microseconds.p50_us,
            p90_us: snapshot.latency_microseconds.p90_us,
            p99_us: snapshot.latency_microseconds.p99_us,
            ring_buffer_occupancy: buffer_stats.current_occupancy,
            ring_buffer_total_pushed: buffer_stats.total_pushed,
            cache_total_keys: cache_stats.total_keys,
            cache_hit_ratio_pct: cache_stats.hit_ratio_pct,
        }
    }
}
