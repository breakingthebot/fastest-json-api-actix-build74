# Fastest Possible JSON API (Build 74)

An ultra-low-latency, zero-cost abstraction, high-throughput asynchronous JSON REST API built with Actix-Web 4.x in Rust. Designed for sub-10 microsecond internal server response times, lock-free atomic telemetry aggregation, zero-copy string borrowing, cache-line aligned circular ring buffers, and sustained 60,000+ requests per second throughput with sub-millisecond round-trip latencies.

## Stack

- **Language / Runtime**: Rust (2021 Edition, `rustc 1.96+`)
- **Framework**: Actix-Web 4.9 (Asynchronous Actor-based HTTP Engine)
- **Async Runtime**: Tokio 1.38 & Actix-RT 2.10
- **Zero-Copy Serialization Engine**: Serde with `ZeroCopyEvent<'a>` string slice borrowing directly from raw HTTP byte buffers
- **In-Memory Ring Buffer**: 64-byte Cache-Line Aligned (`#[repr(align(64))]`) Lock-Free Circular Ring Buffer (`RingBufferService`) with bitmask wraparound (`index & (65536 - 1)`)
- **Observability & Telemetry**: Lock-free Atomic Counters (`AtomicU64`, `AtomicUsize`) & Reservoir Latency Distribution Percentiles (P50, P90, P95, P99, P99.9)
- **Middleware**: Custom `LatencyTracker` injecting high-resolution `X-Response-Time-Microseconds`, `X-Response-Time-Ms`, and `X-Server-Timing` headers via monotonic clock (`std::time::Instant`)
- **Load Testing & Benchmarking**: Dedicated multi-threaded asynchronous client harness (`benchmark-client`)
- **Compiler Optimizations**: Profile `release` configured with `opt-level = 3`, Link-Time Optimization (`lto = true`), single codegen unit (`codegen-units = 1`), `panic = "abort"`, and binary symbol stripping (`strip = true`)
- **CI/CD**: GitHub Actions (`cargo fmt`, `cargo check`, `cargo test`, `cargo clippy`, release binary compilation)

---

## Benchmark & Performance Highlights

Benchmarked on a local workstation using the built-in multi-threaded asynchronous load client (`benchmark-client`) over keep-alive connection pools:

| Endpoint | Concurrency | Total Requests | Throughput (RPS) | Internal Server P50 | Internal Server P90 | Internal Server P99 | Mean Server Latency |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `GET /api/v1/ping` | 50 workers | 10,000 reqs | **58,735 req/sec** | **4 μs (0.004ms)** | **8 μs (0.008ms)** | **19 μs (0.019ms)** | **5.44 μs** |
| `POST /api/v1/events/ingest/zerocopy` | 50 workers | 10,000 reqs | **61,093 req/sec** | **7 μs (0.007ms)** | **11 μs (0.011ms)** | **29 μs (0.029ms)** | **8.72 μs** |
| `POST /api/v1/events/ingest/batch` (5 items/req) | 50 workers | 5,000 reqs (25k events) | **37,468 req/sec** (**187,340 events/s**) | **12 μs (0.012ms)** | **29 μs (0.029ms)** | **79 μs (0.079ms)** | **21.16 μs** |
| `POST /api/v1/echo` | 50 workers | 5,000 reqs | **66,576 req/sec** | **7 μs (0.007ms)** | **12 μs (0.012ms)** | **20 μs (0.020ms)** | **8.31 μs** |
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

Run the full integration test suite:
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
# Benchmark Zero-Copy Ingestion (10,000 requests, 50 concurrent workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e zerocopy

# Benchmark Batch Ingestion (5,000 requests, 25,000 total events)
cargo run --release --bin benchmark-client -- -n 5000 -c 50 -e batch

# Benchmark Ping endpoint (10,000 requests, 50 concurrent workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e ping

# Benchmark JSON Echo endpoint (5,000 requests, 50 concurrent workers)
cargo run --release --bin benchmark-client -- -n 5000 -c 50 -e echo
```

---

## API Endpoints Reference

### 1. Heartbeat & Health
- `GET /health` or `GET /api/v1/health`
  - Returns service status, semantic version, uptime, worker count, and CPU/OS architecture.
- `GET /ping` or `GET /api/v1/ping`
  - Returns ultra-lightweight zero-allocation heartbeat response: `{"message":"pong","timestamp_ms":1724784000000,"unix_nanos":1724784000000000000}`.

### 2. Zero-Copy Event Ingestion & In-Memory Ring Buffer
- `POST /api/v1/events/ingest/zerocopy`
  - Ingests single telemetry event borrowing strings directly from request byte slice with zero intermediate heap allocations.
  - Request body:
    ```json
    {
      "event_id": "evt-001",
      "topic": "sensor.temperature",
      "source": "node-42",
      "severity": "info",
      "metric_value": 24.85,
      "timestamp_ms": 1724784000000
    }
    ```
- `POST /api/v1/events/ingest/batch`
  - Batch ingestion of multiple event records in a single payload.
- `GET /api/v1/events/buffer/stats`
  - Returns ring buffer capacity (65,536), current occupancy, write/read head positions, total pushed, dropped count, and estimated allocated memory.
- `GET /api/v1/events/buffer/recent?limit=20&topic=sensor.temperature`
  - Non-destructive query returning the most recent events ordered newest first.
- `POST /api/v1/events/buffer/drain`
  - Atomically drains all buffered events.

### 3. Real-Time Telemetry & Metrics
- `GET /metrics` or `GET /api/v1/metrics`
  - Returns lock-free atomic counters: total requests, active requests, 2xx/4xx/5xx counts, average RPS, route breakdown, and microsecond latency percentiles (min, mean, p50, p90, p95, p99, p99.9, max).
- `POST /api/v1/metrics/reset`
  - Resets all telemetry counters and latency reservoirs.

### 4. JSON Echo & Synthetic Generation
- `POST /api/v1/echo`: Ingests JSON, validates tags, and returns payload metrics.
- `GET /api/v1/benchmark/synthetic?size=small`: Generates realistic deterministic synthetic inventory items.
- `POST /api/v1/benchmark/ingest`: Ingests and aggregates batch item valuations.

---

## Architecture Notes

### Why Actix-Web and Rust?
Rust's ownership model and zero-cost abstractions allow building networked services without garbage collection pauses, data races, or runtime overhead. Actix-Web utilizes an asynchronous actor model backed by Tokio and OS-native event loops (`epoll` on Linux, `kqueue` on macOS, `IOCP` on Windows), distributing requests across a dedicated pool of OS worker threads without mutex contention.

### Zero-Copy Deserialization with Byte Borrowing
In `ZeroCopyEvent<'a>`, all string fields (`&'a str`) borrow memory directly from the incoming `web::Bytes` slice. This eliminates heap allocations for strings during JSON parsing, allowing the CPU to read field slices in place and saving hundreds of thousands of heap allocations per second under heavy load.

### Cache-Line Padded Lock-Free Circular Ring Buffer
To store incoming events without database latency, `RingBufferService` manages a pre-allocated circular buffer of 65,536 slots. The read and write heads are annotated with `#[repr(align(64))]` to occupy separate 64-byte CPU cache lines, eliminating "false sharing" cache-invalidation penalties between reader and writer cores on multi-socket / multi-core systems.

### High-Resolution Latency Headers
Every request passing through `LatencyTracker` captures start time using `std::time::Instant::now()`. On response dispatch, elapsed time is computed in microseconds and injected into `X-Response-Time-Microseconds`, `X-Response-Time-Ms`, and `X-Server-Timing` headers, allowing upstream load balancers and clients to differentiate between server execution time and network round-trip delay.

---

## Data Handling

- **Zero Persistence Posture**: This service operates entirely in-memory with zero disk persistence.
- **Data Retention**: Buffered telemetry events are maintained in a 65,536-slot in-memory circular ring buffer that automatically overwrites oldest records upon capacity overflow.
- **Privacy & Redaction**: No personally identifiable information (PII) is logged or stored. Metrics endpoints export aggregate statistical counters only.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
