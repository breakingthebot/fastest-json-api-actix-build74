//! src/models/mod.rs
//! Models module exporting domain schemas and DTOs.
//! Connects to: src/models/*.rs, src/handlers/*.rs, src/services/*.rs
//! Created: 2026-08-27

pub mod benchmark;
pub mod cache;
pub mod echo;
pub mod error_response;
pub mod event;
pub mod health;
pub mod metrics;
pub mod ping;
pub mod rate_limit;
pub mod tracing;
pub mod wal;
pub mod websocket;

pub use benchmark::{BenchmarkItem, BenchmarkResponse, IngestRequest, IngestResponse, ItemTelemetry};
pub use cache::{
    BatchCacheItem, BatchSetCacheRequest, BatchSetCacheResponse, CacheItemResponse,
    CacheStatsResponse, SetCacheRequest, ShardStats,
};
pub use echo::{EchoRequest, EchoResponse};
pub use error_response::ApiErrorResponse;
pub use event::{
    BatchEventItem, BatchIngestRequest, BatchIngestResponse, BufferStatsResponse,
    EventIngestResponse, IngestEvent, RecentEventsQuery, RecentEventsResponse, ZeroCopyEvent,
};
pub use health::{HealthResponse, SystemMetadata};
pub use metrics::{EndpointMetrics, LatencyDistribution, MetricsSnapshot};
pub use ping::PingResponse;
pub use rate_limit::{RateLimitDecision, RateLimitErrorResponse, RateLimitStatsResponse};
pub use tracing::{TraceContext, TraceInspectionResponse};
pub use wal::{WalCheckpointResponse, WalStatsResponse, WalSyncResponse};
pub use websocket::{LiveTelemetryFrame, WsClientCommand, WsCommandResponse};
