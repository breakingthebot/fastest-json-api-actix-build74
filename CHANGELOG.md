# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-08-28

### Added
- **64-Way Sharded In-Memory Cache Engine**:
  - `ShardedCacheService` partitioning in-memory key-value storage across 64 independent shards using FNV-1a hash distribution (`(hash ^ (hash >> 16)) & 63`).
  - Sub-microsecond TTL evaluation and lazy eviction on key access.
  - Granular per-shard `RwLock` isolation eliminating global lock contention under heavy concurrency.
  - Achieved **72,852 req/sec** on `GET` operations with **6 μs** P50 server processing time, and **63,854 req/sec** on `PUT` operations with **11 μs** P50.
- **Cache HTTP REST Endpoints**:
  - `GET /api/v1/cache/{key}` retrieving value, shard index, hit count, and remaining TTL milliseconds.
  - `PUT /api/v1/cache/{key}` inserting or updating keys with optional `ttl_seconds`.
  - `DELETE /api/v1/cache/{key}` removing keys from designated shard.
  - `POST /api/v1/cache/batch/set` high-speed batch setting of multiple key-value items.
  - `GET /api/v1/cache/stats` tracking hit ratio percentage, total gets/sets/deletes, memory estimation, and per-shard key distribution.
  - `POST /api/v1/cache/clear` clearing all 64 shards.
  - `POST /api/v1/cache/purge-expired` running an on-demand sweeper across all shards.
- **Benchmark Tool Updates**:
  - Added `-e cache_get` and `-e cache_set` presets with automatic cache pre-population.
- **Comprehensive Integration Tests**:
  - `tests/cache_service_tests.rs` verifying 64-way shard distribution, TTL expiration, hit tracking, batch sets, and clearing.
  - `tests/cache_api_tests.rs` verifying full HTTP CRUD flow, batch setting, 404 responses, and cache telemetry.

## [0.2.0] - 2026-08-28

### Added
- **Zero-Copy Byte Slice Deserialization**:
  - `ZeroCopyEvent<'a>` schema with zero-allocation `&'a str` string borrowing directly from raw incoming `web::Bytes` buffers.
  - `POST /api/v1/events/ingest/zerocopy` endpoint achieving **61,093 req/sec** throughput with **7 μs** P50 server processing time.
- **Cache-Line Aligned Circular Ring Buffer**:
  - `RingBufferService` managing a pre-allocated 65,536-element contiguous memory buffer with power-of-two bitmask indexing (`index & MASK`).
  - Cache-line padding (`#[repr(align(64))]`) on atomic head and tail pointers to prevent CPU false sharing between reader and writer threads.
  - Non-blocking slot updates with overwrite-on-overflow logic and atomic dropped event telemetry.
- **Batch Event Ingestion & Buffer Inspection**:
  - `POST /api/v1/events/ingest/batch` endpoint sustaining **37,468 req/sec** (**187,340 events/sec**) for multi-event batch payloads.
  - `GET /api/v1/events/buffer/stats` returning buffer capacity, live occupancy, head positions, total pushed, and dropped counts.
  - `GET /api/v1/events/buffer/recent` non-destructive query returning recent events ordered newest first with optional topic filtering.
  - `POST /api/v1/events/buffer/drain` atomically extracting buffered records.

## [0.1.0] - 2026-08-27

### Added
- **Core Server Engine**: Asynchronous HTTP engine powered by Actix-Web 4.9 and Tokio runtime with multi-threaded worker pools, keep-alive tuning, and socket backlog configuration.
- **Ultra-Fast Heartbeat Endpoints**:
  - `GET /health` and `GET /api/v1/health` with CPU architecture, runtime environment, worker thread counts, and uptime metadata.
  - `GET /ping` and `GET /api/v1/ping` zero-allocation heartbeat endpoint returning sub-10 microsecond responses.
- **Lock-Free Atomic Telemetry**:
  - `MetricsService` tracking total requests, active requests, 2xx/4xx/5xx status counts, total bytes sent, and route distributions using `AtomicU64` and `AtomicUsize`.
  - Latency reservoir sampling with automatic percentile computation (Min, Mean, P50, P90, P95, P99, P99.9, Max) in microseconds.
  - `GET /api/v1/metrics` and `POST /api/v1/metrics/reset` endpoints.
- **JSON Serialization & Echo Engine**:
  - `POST /api/v1/echo` endpoint measuring serialization speed, payload byte sizes, and tag structures.
  - Custom `JsonConfig` with standard RFC 7807 problem details error responder for malformed payloads.
- **Synthetic Benchmark & Batch Ingestion**:
  - `GET /api/v1/benchmark/synthetic` generating deterministic simulated e-commerce telemetry items with customizable batch sizes (small, medium, large, xlarge).
  - `POST /api/v1/benchmark/ingest` validating batch records and aggregating item inventory valuations.
- **Custom Latency Middleware**:
  - `LatencyTracker` measuring sub-millisecond execution times and injecting `X-Response-Time-Microseconds`, `X-Response-Time-Ms`, `X-Server-Timing`, and `Server` headers.
- **Dedicated Benchmarking Client**:
  - Standalone multi-threaded asynchronous load client (`src/bin/benchmark_client.rs`) supporting configurable concurrency, request counts, endpoint presets, and statistical percentile reporting.
- **Comprehensive Test Suite & CI/CD Pipeline**:
  - Integration tests covering all components and GitHub Actions CI workflow.
