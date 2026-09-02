# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-08-28

### Added
- **64-Way Sharded Token Bucket Rate Limiter**:
  - `RateLimiterService` managing 64 independent token bucket partitions to prevent lock contention on client IP evaluations.
  - Sub-500 nanosecond token acquisition with fractional time-delta replenishment (`tokens = min(capacity, tokens + elapsed * rate)`).
  - Default 1,000 burst capacity with 500 requests/sec refill rate per client.
- **DDoS Protection & RFC Rate Limit Headers Middleware**:
  - `RateLimitMiddleware` intercepting requests, identifying client IP (`X-Forwarded-For`, `X-Real-IP`, or socket peer IP), and injecting standard headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, and `X-RateLimit-Reset`.
  - Structured HTTP 429 Too Many Requests response with `Retry-After` header and RFC 7807 problem details JSON payload when capacity is exceeded.
  - Built-in route whitelist bypassing rate limits for heartbeat (`/ping`), health (`/health`), Prometheus metrics (`/metrics`), WebSocket streams (`/ws/metrics`), and dashboard UI (`/dashboard`).
- **Rate Limiter Telemetry & Reset Endpoints**:
  - `GET /api/v1/ratelimit/stats` tracking total evaluated, allowed, rejected, active client buckets, and rejection percentage.
  - `POST /api/v1/ratelimit/reset` resetting client token buckets.
- **Integration Tests**:
  - `tests/rate_limiter_tests.rs` verifying token replenishment math, burst depletion, and active bucket tracking.
  - `tests/rate_limit_middleware_tests.rs` verifying HTTP 429 status codes, header injection, and whitelist bypass behavior.

## [0.6.0] - 2026-08-28

### Added
- **Append-Only Binary Write-Ahead Log (WAL) Engine**:
  - `WalService` persisting event payloads with a 12-byte framed binary header layout: `[MAGIC(4)][LEN(4)][CRC32(4)][PAYLOAD]`.
  - SIMD-accelerated hardware CRC32 frame checksum calculation and verification via `crc32fast`.
  - Automatic startup crash recovery replaying uncorrupted WAL log records into `RingBufferService` on server boot.
  - Granular `POST /api/v1/wal/sync` for synchronous physical `fsync` flushes and `POST /api/v1/wal/checkpoint` for log rotation and truncation.
  - `GET /api/v1/wal/stats` returning file size on disk, total binary bytes written, appends, and skipped corrupted frames.
- **Integration Tests**:
  - `tests/wal_tests.rs`.

## [0.5.0] - 2026-08-28

### Added
- **WebSocket Real-Time Telemetry Streaming Pipeline**:
  - `WebSocketBroadcaster` service streaming continuous 100ms telemetry frames (`LiveTelemetryFrame`) over Tokio broadcast channels to connected WebSocket clients.
  - Bidirectional WebSocket communication on `GET /ws/metrics` and `GET /api/v1/stream/metrics` supporting client commands (`ping`, `get_snapshot`, `reset_metrics`, `drain_buffer`).
- **Embedded Real-Time Monitoring Dashboard**:
  - `GET /dashboard` and `GET /api/v1/stream/dashboard` serving a self-contained zero-dependency web monitoring UI.

## [0.4.0] - 2026-08-28

### Added
- **Prometheus / OpenMetrics 0.0.4 Text Exposition**:
  - `render_prometheus_metrics` engine rendering live system counters, gauges, quantile summaries, ring buffer capacity, and sharded cache metrics into standard Prometheus format at `GET /metrics`.
- **W3C Distributed Tracing Pipeline**:
  - `TracingMiddleware` implementing W3C Trace Context specification (`traceparent`, `tracestate`, `X-Trace-Id`, `X-Span-Id`).

## [0.3.0] - 2026-08-28

### Added
- **64-Way Sharded In-Memory Cache Engine**:
  - `ShardedCacheService` partitioning in-memory key-value storage across 64 independent shards using FNV-1a hash distribution (`(hash ^ (hash >> 16)) & 63`).
  - Sub-microsecond TTL evaluation and lazy eviction on key access.

## [0.2.0] - 2026-08-28

### Added
- **Zero-Copy Byte Slice Deserialization**:
  - `ZeroCopyEvent<'a>` schema with zero-allocation `&'a str` string borrowing directly from raw incoming `web::Bytes` buffers.
- **Cache-Line Aligned Circular Ring Buffer**:
  - `RingBufferService` managing a pre-allocated 65,536-element contiguous memory buffer with power-of-two bitmask indexing.

## [0.1.0] - 2026-08-27

### Added
- **Core Server Engine**: Asynchronous HTTP engine powered by Actix-Web 4.9 and Tokio runtime with multi-threaded worker pools, keep-alive tuning, and socket backlog configuration.
- **Ultra-Fast Heartbeat Endpoints**: `GET /health`, `GET /api/v1/health`, `GET /ping`, `GET /api/v1/ping`.
- **Lock-Free Atomic Telemetry & Latency Reservoir Percentiles**.
