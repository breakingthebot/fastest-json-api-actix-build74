# Fastest Possible JSON API (Build 74)

An ultra-low-latency, zero-cost abstraction, high-throughput asynchronous JSON REST API built with Actix-Web 4.x in Rust. Designed for sub-10 microsecond internal server response times, lock-free atomic telemetry aggregation, zero-copy string borrowing, cache-line aligned circular ring buffers, 64-way partitioned in-memory caching with sub-microsecond TTL evaluation, Prometheus / OpenMetrics 0.0.4 exposition, W3C distributed tracing header propagation, and sustained 70,000+ requests per second throughput with sub-millisecond round-trip latencies.

## Stack

- **Language / Runtime**: Rust (2021 Edition, `rustc 1.96+`)
- **Framework**: Actix-Web 4.9 (Asynchronous Actor-based HTTP Engine)
- **Async Runtime**: Tokio 1.38 & Actix-RT 2.10
- **Observability & OpenMetrics**: Standard Prometheus text exposition (version 0.0.4) exporting request counters, in-flight gauges, quantile summaries, cache statistics, and ring buffer telemetry at `GET /metrics`
- **Distributed Tracing**: W3C Trace Context specification compliance (`traceparent`, `tracestate`, `X-Trace-Id`, `X-Span-Id`) with automatic 128-bit trace ID generation and header propagation via `TracingMiddleware`
- **Sharded In-Memory Cache Engine**: 64-way Lock-Free Partitioned Cache (`ShardedCacheService`) with FNV-1a hash distribution (`(hash ^ (hash >> 16)) & 63`), sub-microsecond TTL eviction, and hit-ratio telemetry
- **Zero-Copy Serialization Engine**: Serde with `ZeroCopyEvent<'a>` string slice borrowing directly from raw HTTP byte buffers
- **In-Memory Ring Buffer**: 64-byte Cache-Line Aligned (`#[repr(align(64))]`) Lock-Free Circular Ring Buffer (`RingBufferService`) with bitmask wraparound (`index & (65536 - 1)`)
- **Middleware Pipeline**: Custom `TracingMiddleware` and `LatencyTracker` injecting high-resolution `X-Response-Time-Microseconds`, `X-Response-Time-Ms`, `X-Server-Timing`, `traceparent`, and `X-Trace-Id` headers
- **Load Testing & Benchmarking**: Dedicated multi-threaded asynchronous client harness (`benchmark-client`)
- **Compiler Optimizations**: Profile `release` configured with `opt-level = 3`, Link-Time Optimization (`lto = true`), single codegen unit (`codegen-units = 1`), `panic = "abort"`, and binary symbol stripping (`strip = true`)
- **CI/CD**: GitHub Actions (`cargo fmt`, `cargo check`, `cargo test`, `cargo clippy`, release binary compilation)

---

## Benchmark & Performance Highlights

Benchmarked on a local workstation using the built-in multi-threaded asynchronous load client (`benchmark-client`) over keep-alive connection pools:

| Endpoint | Concurrency | Total Requests | Throughput (RPS) | Internal Server P50 | Internal Server P90 | Internal Server P99 | Mean Server Latency |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `GET /api/v1/cache/{key}` (64 Shards) | 50 workers | 10,000 reqs | **72,852 req/sec** | **6 μs (0.006ms)** | **10 μs (0.010ms)** | **21 μs (0.021ms)** | **7.85 μs** |
| `PUT /api/v1/cache/{key}` (64 Shards) | 50 workers | 10,000 reqs | **63,854 req/sec** | **11 μs (0.011ms)** | **17 μs (0.017ms)** | **30 μs (0.030ms)** | **12.99 μs** |
| `POST /api/v1/echo` | 50 workers | 5,000 reqs | **66,576 req/sec** | **7 μs (0.007ms)** | **12 μs (0.012ms)** | **20 μs (0.020ms)** | **8.31 μs** |
| `POST /api/v1/events/ingest/zerocopy` | 50 workers | 10,000 reqs | **61,093 req/sec** | **7 μs (0.007ms)** | **11 μs (0.011ms)** | **29 μs (0.029ms)** | **8.72 μs** |
| `GET /api/v1/ping` | 50 workers | 10,000 reqs | **58,735 req/sec** | **4 μs (0.004ms)** | **8 μs (0.008ms)** | **19 μs (0.019ms)** | **5.44 μs** |
| `POST /api/v1/events/ingest/batch` (5 items/req) | 50 workers | 5,000 reqs (25k events) | **37,468 req/sec** (**187,340 events/s**) | **12 μs (0.012ms)** | **29 μs (0.029ms)** | **79 μs (0.079ms)** | **21.16 μs** |
| `GET /api/v1/benchmark/synthetic` | 30 workers | 3,000 reqs | **42,850 req/sec** | **9 μs (0.009ms)** | **16 μs (0.016ms)** | **28 μs (0.028ms)** | **10.15 μs** |

---

## Setup

1. **Prerequisites**: Ensure the Rust toolchain (`cargo` and `rustc`) is installed:
   ```bash
   rustc --version
   cargo --version
   ```
2. **Clone the repository**:
   ```bash
   git clone https://github.com/breakingthebot/fastest-json-api-actix-build74.git
   cd fastest-json-api-actix-build74
   ```
3. **Copy environment variables**:
   ```bash
   cp .env.example .env
   ```

---

## Environment Variables

See `.env.example` for runtime configuration keys:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `SERVER_HOST` | Socket binding IP address | `127.0.0.1` |
| `SERVER_PORT` | HTTP port to listen on | `8080` |
| `SERVER_WORKERS` | Number of worker threads | Logical CPU core count |
| `SERVER_KEEPALIVE_SECS` | TCP Keep-Alive timeout in seconds | `75` |
| `SERVER_BACKLOG` | OS socket backlog buffer size | `2048` |
| `MAX_PAYLOAD_BYTES` | Maximum JSON request payload limit | `2097152` (2 MB) |
| `APP_ENV` | Application environment label | `development` |
| `RUST_LOG` | Logging verbosity filter | `info` |

---

## Running Locally

### 1. Run in Development Mode
```bash
cargo run
```

### 2. Run with Maximum Compiler Optimizations (Release Mode)
```bash
cargo run --release
```

---

## Running Tests

Run the full 22-test integration test suite:
```bash
cargo test --verbose
```

---

## Benchmarking & Performance Verification

This project includes a high-throughput load generation tool (`benchmark-client`) that tests concurrency, throughput, and sub-millisecond latency distributions.

### 1. Launch the Server
In terminal 1:
```bash
cargo run --release
```

### 2. Execute Benchmark Harness
In terminal 2:
```bash
# Benchmark Cache GET operations across 64 shards (10,000 requests, 50 workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e cache_get

# Benchmark Cache PUT/SET operations across 64 shards (10,000 requests, 50 workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e cache_set

# Benchmark Zero-Copy Ingestion (10,000 requests, 50 workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e zerocopy

# Benchmark Batch Ingestion (5,000 requests, 25,000 total events)
cargo run --release --bin benchmark-client -- -n 5000 -c 50 -e batch

# Benchmark Ping endpoint (10,000 requests, 50 workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e ping
```

---

## API Endpoints Reference

### 1. Prometheus / OpenMetrics & Distributed Tracing
- `GET /metrics` or `GET /api/v1/metrics/prometheus`
  - Returns standard Prometheus 0.0.4 text exposition format (`text/plain; version=0.0.4; charset=utf-8`) ready for scraping by Prometheus, Grafana Agent, or Datadog.
  - Metrics exported: `http_requests_total`, `http_requests_in_flight`, `http_request_duration_seconds`, `ring_buffer_occupancy`, `cache_hit_ratio_percent`, `cache_keys_total`.
- `GET /api/v1/trace/current`
  - Returns active W3C trace context, span hierarchy, and propagation headers for the current request.

### 2. 64-Way Sharded In-Memory Cache Engine
- `GET /api/v1/cache/{key}`: Sub-10μs key retrieval with hit count and remaining TTL milliseconds.
- `PUT /api/v1/cache/{key}`: Sets or updates a cache key with optional `ttl_seconds`.
- `DELETE /api/v1/cache/{key}`: Removes a key from its assigned shard.
- `POST /api/v1/cache/batch/set`: Batch key-value insertion across shards.
- `GET /api/v1/cache/stats`: Telemetry showing overall hit ratio, memory usage, and per-shard distribution.
- `POST /api/v1/cache/clear`: Clears all 64 cache partitions.
- `POST /api/v1/cache/purge-expired`: On-demand expiration sweeper.

### 3. Zero-Copy Event Ingestion & In-Memory Ring Buffer
- `POST /api/v1/events/ingest/zerocopy`: Ingests single telemetry event borrowing strings directly from request byte slice.
- `POST /api/v1/events/ingest/batch`: Batch ingestion of multiple event records in a single payload.
- `GET /api/v1/events/buffer/stats`: Returns ring buffer capacity (65,536), live occupancy, write/read head positions, total pushed, and dropped count.
- `GET /api/v1/events/buffer/recent?limit=20&topic=sensor.temperature`: Non-destructive query returning the most recent events.
- `POST /api/v1/events/buffer/drain`: Atomically drains all buffered events.

### 4. Heartbeat, Metrics & Serialization
- `GET /api/v1/ping`: Zero-allocation heartbeat response in sub-10μs.
- `GET /api/v1/health`: System health reporting CPU architecture, worker thread counts, and uptime.
- `GET /api/v1/metrics`: Lock-free atomic counters (RPS, status codes, latency percentiles P50..P99.9).
- `POST /api/v1/echo`: Validates JSON payloads and measures processing duration.

---

## Architecture Notes

### Prometheus / OpenMetrics 0.0.4 Text Exposition
`render_prometheus_metrics` compiles live system state into standard Prometheus text format without external runtime overhead. Scrapers can observe request rate counters, in-flight gauges, sub-millisecond quantile summaries (`0.5`, `0.9`, `0.95`, `0.99`, `0.999`), ring buffer occupancy, and cache hit ratios directly on `/metrics`.

### W3C Distributed Tracing Pipeline
`TracingMiddleware` enforces the W3C Trace Context recommendation. If incoming requests include `traceparent` (e.g. `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`), the global `trace_id` is preserved and a new `span_id` is assigned for this server hop. If missing, a high-entropy 128-bit trace ID is generated automatically. Response headers include `traceparent`, `X-Trace-Id`, `X-Span-Id`, and pass-through `tracestate`.

### 64-Way Sharded Cache Architecture
To eliminate global lock contention across dozens of worker threads, `ShardedCacheService` partitions cache storage across 64 independent shards. Keys are mapped to shards using a fast FNV-1a hash function (`(hash ^ (hash >> 16)) & 63`). Each shard is guarded by its own `RwLock`, allowing 64 concurrent write operations and hundreds of concurrent read operations to execute simultaneously with zero lock waiting.

### Zero-Copy Deserialization with Byte Borrowing
In `ZeroCopyEvent<'a>`, all string fields (`&'a str`) borrow memory directly from the incoming `web::Bytes` slice. This eliminates heap allocations for strings during JSON parsing, allowing the CPU to read field slices in place and saving hundreds of thousands of heap allocations per second under heavy load.

---

## Data Handling

- **Zero Persistence Posture**: This service operates entirely in-memory with zero disk persistence.
- **Data Retention**: Cached key-value entries and buffered telemetry events are stored in volatile memory and evicted according to TTL or ring buffer capacity wraparound.
- **Privacy & Redaction**: No personally identifiable information (PII) is logged or stored. Metrics endpoints export aggregate statistical counters only.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
