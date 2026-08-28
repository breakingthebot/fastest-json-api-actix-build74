# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-08-28

### Added
- **Prometheus / OpenMetrics 0.0.4 Text Exposition**:
  - `render_prometheus_metrics` engine rendering live system counters, gauges, quantile summaries, ring buffer capacity, and sharded cache metrics into standard Prometheus format.
  - Exposed at `GET /metrics` and `GET /api/v1/metrics/prometheus` with `text/plain; version=0.0.4; charset=utf-8` header.
- **W3C Distributed Tracing Pipeline**:
  - `TracingMiddleware` implementing the W3C Trace Context specification (`traceparent`, `tracestate`, `X-Trace-Id`, `X-Span-Id`).
  - Automated 128-bit trace ID and 64-bit span ID generation with context propagation to request extensions and response headers.
  - `GET /api/v1/trace/current` debug inspection endpoint returning active trace context and span hierarchy.
- **Comprehensive Integration Tests**:
  - `tests/prometheus_tests.rs` verifying HELP/TYPE declarations, quantiles, and content-type headers.
  - `tests/tracing_tests.rs` verifying trace ID generation, parent span propagation, and `tracestate` pass-through.

## [0.3.0] - 2026-08-28

### Added
- **64-Way Sharded In-Memory Cache Engine**:
  - `ShardedCacheService` partitioning in-memory key-value storage across 64 independent shards using FNV-1a hash distribution (`(hash ^ (hash >> 16)) & 63`).
  - Sub-microsecond TTL evaluation and lazy eviction on key access.
  - Granular per-shard `RwLock` isolation eliminating global lock contention under heavy concurrency.
  - Achieved **72,852 req/sec** on `GET` operations with **6 μs** P50 server processing time, and **63,854 req/sec** on `PUT` operations with **11 μs** P50.
- **Cache HTTP REST Endpoints**:
  - `GET /api/v1/cache/{key}`, `PUT /api/v1/cache/{key}`, `DELETE /api/v1/cache/{key}`, `POST /api/v1/cache/batch/set`.
  - `GET /api/v1/cache/stats`, `POST /api/v1/cache/clear`, `POST /api/v1/cache/purge-expired`.

## [0.2.0] - 2026-08-28

### Added
- **Zero-Copy Byte Slice Deserialization**:
  - `ZeroCopyEvent<'a>` schema with zero-allocation `&'a str` string borrowing directly from raw incoming `web::Bytes` buffers.
  - `POST /api/v1/events/ingest/zerocopy` endpoint achieving **61,093 req/sec** throughput with **7 μs** P50 server processing time.
- **Cache-Line Aligned Circular Ring Buffer**:
  - `RingBufferService` managing a pre-allocated 65,536-element contiguous memory buffer with power-of-two bitmask indexing (`index & MASK`).
  - Cache-line padding (`#[repr(align(64))]`) on atomic head and tail pointers to prevent CPU false sharing.
- **Batch Event Ingestion & Buffer Inspection**:
  - `POST /api/v1/events/ingest/batch` endpoint sustaining **37,468 req/sec** (**187,340 events/sec**).
  - `GET /api/v1/events/buffer/stats`, `GET /api/v1/events/buffer/recent`, `POST /api/v1/events/buffer/drain`.

## [0.1.0] - 2026-08-27

### Added
- **Core Server Engine**: Asynchronous HTTP engine powered by Actix-Web 4.9 and Tokio runtime with multi-threaded worker pools, keep-alive tuning, and socket backlog configuration.
- **Ultra-Fast Heartbeat Endpoints**: `GET /health`, `GET /api/v1/health`, `GET /ping`, `GET /api/v1/ping`.
- **Lock-Free Atomic Telemetry**: `MetricsService` tracking total requests, active requests, status codes, latency reservoir percentiles (P50..P99.9).
- **JSON Serialization & Echo Engine**: `POST /api/v1/echo`, synthetic benchmarks, and RFC 7807 error responders.
- **Latency Middleware & Benchmark Client**: Custom `LatencyTracker` and CLI tool (`benchmark-client`).
