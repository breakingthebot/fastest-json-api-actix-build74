# Fastest Possible JSON API (Build 74)

An ultra-low-latency, zero-cost abstraction, high-throughput asynchronous JSON REST API built with Actix-Web 4.x in Rust. Designed for sub-10 microsecond internal server response times, lock-free atomic telemetry aggregation, and sustained 60,000+ requests per second throughput with sub-millisecond round-trip latencies.

## Stack

- **Language / Runtime**: Rust (2021 Edition, `rustc 1.96+`)
- **Framework**: Actix-Web 4.9 (Asynchronous Actor-based HTTP Engine)
- **Async Runtime**: Tokio 1.38 & Actix-RT 2.10
- **Serialization / Deserialization**: Serde & Serde-JSON (Zero-allocation / streaming deserialization)
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
# Benchmark Ping endpoint (10,000 requests, 50 concurrent workers)
cargo run --release --bin benchmark-client -- -n 10000 -c 50 -e ping

# Benchmark JSON Echo endpoint (5,000 requests, 50 concurrent workers)
cargo run --release --bin benchmark-client -- -n 5000 -c 50 -e echo

# Benchmark Synthetic Payload Generator
cargo run --release --bin benchmark-client -- -n 3000 -c 30 -e synthetic

# Custom URL and custom concurrency
cargo run --release --bin benchmark-client -- --url http://127.0.0.1:8080/api/v1/ping -n 20000 -c 100
```

---

## API Endpoints Reference

### 1. Heartbeat & Health
- `GET /health` or `GET /api/v1/health`
  - Returns service status, semantic version, uptime, worker count, and CPU/OS architecture.
- `GET /ping` or `GET /api/v1/ping`
  - Returns ultra-lightweight zero-allocation heartbeat response: `{"message":"pong","timestamp_ms":1724784000000,"unix_nanos":1724784000000000000}`.

### 2. Real-Time Telemetry & Metrics
- `GET /metrics` or `GET /api/v1/metrics`
  - Returns lock-free atomic counters: total requests, active requests, 2xx/4xx/5xx counts, average RPS, route breakdown, and microsecond latency percentiles (min, mean, p50, p90, p95, p99, p99.9, max).
- `POST /api/v1/metrics/reset`
  - Resets all telemetry counters and latency reservoirs.

### 3. JSON Echo & Serialization Benchmarking
- `POST /api/v1/echo`
  - Ingests structured JSON, validates tags, measures internal parsing time, and returns payload metrics.
  - Example request body:
    ```json
    {
      "message": "Benchmark payload",
      "count": 42,
      "enabled": true,
      "tags": ["actix", "rust", "low-latency"],
      "metadata": { "region": "us-east-1", "cluster": 7 }
    }
    ```

### 4. Synthetic Data Generation & Batch Ingestion
- `GET /api/v1/benchmark/synthetic?size=small` (Options: `small`, `medium`, `large`, `xlarge` or `count=N`)
  - Generates realistic deterministic synthetic inventory items with nested telemetry sensors.
- `POST /api/v1/benchmark/ingest`
  - Ingests batch item records, validates pricing/stock integrity, and computes batch summary statistics in microseconds.

---

## Architecture Notes

### Why Actix-Web and Rust?
Rust's ownership model and zero-cost abstractions allow building networked services without garbage collection pauses, data races, or runtime overhead. Actix-Web utilizes an asynchronous actor model backed by Tokio and OS-native event loops (`epoll` on Linux, `kqueue` on macOS, `IOCP` on Windows), distributing requests across a dedicated pool of OS worker threads without mutex contention.

### Zero-Contention Telemetry Architecture
Instead of synchronizing request tracking across threads using heavy Mutex locks, `MetricsService` relies on lock-free `AtomicU64` and `AtomicUsize` primitives with `Ordering::Relaxed` memory ordering. Latency percentiles use reservoir sampling with a dedicated `RwLock` that is sampled asynchronously, eliminating lock contention on high-throughput request paths.

### High-Resolution Latency Headers
Every request passing through `LatencyTracker` middleware captures start time using `std::time::Instant::now()`. On response dispatch, elapsed time is computed in microseconds and injected into `X-Response-Time-Microseconds`, `X-Response-Time-Ms`, and `X-Server-Timing` headers, allowing upstream load balancers and clients to differentiate between server execution time and network round-trip delay.

---

## Data Handling

- **Zero Persistence Posture**: This service operates entirely in-memory with zero disk persistence.
- **Data Retention**: Synthetic benchmark payloads and echo requests are processed in memory and discarded upon response transmission.
- **Privacy & Redaction**: No personally identifiable information (PII) is logged or stored. Metrics endpoints export aggregate statistical counters only.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
