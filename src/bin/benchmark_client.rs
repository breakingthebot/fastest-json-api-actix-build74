//! src/bin/benchmark_client.rs
//! High-throughput concurrent load testing client and latency analyzer for Actix JSON API.
//! Connects to: Actix Web HTTP server endpoints
//! Created: 2026-08-27

use std::env;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
struct BenchConfig {
    target_url: String,
    total_requests: usize,
    concurrency: usize,
    endpoint_type: String,
}

#[derive(Debug, Clone)]
struct RequestResult {
    status_code: u16,
    client_duration_us: u64,
    server_duration_us: Option<u64>,
    bytes_received: usize,
    is_error: bool,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let mut config = BenchConfig {
        target_url: "http://127.0.0.1:8080/api/v1/ping".to_string(),
        total_requests: 5000,
        concurrency: 50,
        endpoint_type: "ping".to_string(),
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-u" | "--url" => {
                if i + 1 < args.len() {
                    config.target_url = args[i + 1].clone();
                    i += 1;
                }
            }
            "-n" | "--requests" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<usize>() {
                        config.total_requests = n;
                    }
                    i += 1;
                }
            }
            "-c" | "--concurrency" => {
                if i + 1 < args.len() {
                    if let Ok(c) = args[i + 1].parse::<usize>() {
                        config.concurrency = c;
                    }
                    i += 1;
                }
            }
            "-e" | "--endpoint" => {
                if i + 1 < args.len() {
                    config.endpoint_type = args[i + 1].clone();
                    match config.endpoint_type.as_str() {
                        "ping" => config.target_url = "http://127.0.0.1:8080/api/v1/ping".to_string(),
                        "health" => config.target_url = "http://127.0.0.1:8080/api/v1/health".to_string(),
                        "metrics" => config.target_url = "http://127.0.0.1:8080/api/v1/metrics".to_string(),
                        "synthetic" => config.target_url = "http://127.0.0.1:8080/api/v1/benchmark/synthetic?size=small".to_string(),
                        "echo" => config.target_url = "http://127.0.0.1:8080/api/v1/echo".to_string(),
                        _ => {}
                    }
                    i += 1;
                }
            }
            "-h" | "--help" => {
                println!("Actix Ultra-Fast JSON API Benchmark Client");
                println!("Usage: cargo run --bin benchmark-client -- [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -u, --url <URL>            Target URL (default: http://127.0.0.1:8080/api/v1/ping)");
                println!("  -n, --requests <COUNT>     Total number of requests to send (default: 5000)");
                println!("  -c, --concurrency <COUNT>  Concurrent worker tasks (default: 50)");
                println!("  -e, --endpoint <TYPE>      Preset endpoint: 'ping', 'health', 'metrics', 'synthetic', 'echo'");
                println!("  -h, --help                 Display this help message");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    println!("==================================================================");
    println!("⚡ Actix Ultra-Fast JSON API Benchmark Suite");
    println!("   Target URL      : {}", config.target_url);
    println!("   Total Requests  : {}", config.total_requests);
    println!("   Concurrency     : {}", config.concurrency);
    println!("   Endpoint Preset : {}", config.endpoint_type);
    println!("==================================================================");
    println!("Starting load generation...");

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(config.concurrency + 10)
        .tcp_nodelay(true)
        .build()
        .expect("Failed to build HTTP client");

    let client_arc = Arc::new(client);
    let (tx, mut rx) = mpsc::channel::<RequestResult>(config.total_requests + 100);

    let requests_per_worker = config.total_requests / config.concurrency;
    let remainder = config.total_requests % config.concurrency;

    let overall_start = Instant::now();
    let mut handles = Vec::with_capacity(config.concurrency);

    for worker_id in 0..config.concurrency {
        let count = requests_per_worker + if worker_id == 0 { remainder } else { 0 };
        let tx_clone = tx.clone();
        let client = Arc::clone(&client_arc);
        let url = config.target_url.clone();
        let is_echo = config.endpoint_type == "echo";

        let handle = tokio::spawn(async move {
            for _ in 0..count {
                let req_start = Instant::now();

                let res_result = if is_echo {
                    client
                        .post(&url)
                        .json(&serde_json::json!({
                            "message": "benchmark payload",
                            "tags": ["actix", "rust", "bench"],
                            "count": 42
                        }))
                        .send()
                        .await
                } else {
                    client.get(&url).send().await
                };

                let client_elapsed_us = req_start.elapsed().as_micros() as u64;

                match res_result {
                    Ok(response) => {
                        let status_code = response.status().as_u16();
                        let server_duration_us = response
                            .headers()
                            .get("x-response-time-microseconds")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok());

                        let bytes = match response.bytes().await {
                            Ok(b) => b.len(),
                            Err(_) => 0,
                        };

                        let is_error = status_code >= 400;

                        let _ = tx_clone.send(RequestResult {
                            status_code,
                            client_duration_us: client_elapsed_us,
                            server_duration_us,
                            bytes_received: bytes,
                            is_error,
                        }).await;
                    }
                    Err(_) => {
                        let _ = tx_clone.send(RequestResult {
                            status_code: 0,
                            client_duration_us: client_elapsed_us,
                            server_duration_us: None,
                            bytes_received: 0,
                            is_error: true,
                        }).await;
                    }
                }
            }
        });
        handles.push(handle);
    }

    drop(tx); // Close the original sender so receiver drains properly

    let mut results = Vec::with_capacity(config.total_requests);
    while let Some(res) = rx.recv().await {
        results.push(res);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let overall_elapsed = overall_start.elapsed();
    let total_secs = overall_elapsed.as_secs_f64();
    let total_samples = results.len();

    if total_samples == 0 {
        eprintln!("❌ Benchmark failed: No requests were completed.");
        return;
    }

    let mut client_latencies: Vec<u64> = results.iter().map(|r| r.client_duration_us).collect();
    client_latencies.sort_unstable();

    let mut server_latencies: Vec<u64> = results.iter().filter_map(|r| r.server_duration_us).collect();
    server_latencies.sort_unstable();

    let success_count = results.iter().filter(|r| !r.is_error && r.status_code == 200).count();
    let error_count = results.iter().filter(|r| r.is_error).count();
    let total_bytes: usize = results.iter().map(|r| r.bytes_received).sum();

    let rps = (total_samples as f64) / total_secs;
    let mb_per_sec = ((total_bytes as f64) / (1024.0 * 1024.0)) / total_secs;

    let calc_percentile = |list: &[u64], pct: f64| -> u64 {
        if list.is_empty() {
            0
        } else {
            let idx = ((list.len() as f64 * pct) / 100.0).round() as usize;
            let clamped = idx.saturating_sub(1).min(list.len() - 1);
            list[clamped]
        }
    };

    let client_min = *client_latencies.first().unwrap_or(&0);
    let client_max = *client_latencies.last().unwrap_or(&0);
    let client_sum: u64 = client_latencies.iter().sum();
    let client_avg = (client_sum as f64) / (total_samples as f64);

    let client_p50 = calc_percentile(&client_latencies, 50.0);
    let client_p90 = calc_percentile(&client_latencies, 90.0);
    let client_p95 = calc_percentile(&client_latencies, 95.0);
    let client_p99 = calc_percentile(&client_latencies, 99.0);
    let client_p999 = calc_percentile(&client_latencies, 99.9);

    println!();
    println!("📊 ==================== BENCHMARK RESULTS ====================");
    println!("   Total Requests       : {}", total_samples);
    println!("   Successful (200 OK)  : {}", success_count);
    println!("   Failed / Errors      : {}", error_count);
    println!("   Total Time Elapsed   : {:.3} seconds", total_secs);
    println!("   Throughput (RPS)     : {:.2} req/sec", rps);
    println!("   Data Transfer Rate   : {:.2} MB/sec (Total: {:.2} KB)", mb_per_sec, (total_bytes as f64) / 1024.0);
    println!("--------------------------------------------------------------");
    println!("   CLIENT ROUND-TRIP LATENCY (Total Network + Server Time):");
    println!("     Min Latency        : {:>8} μs ({:.3} ms)", client_min, client_min as f64 / 1000.0);
    println!("     Mean Latency       : {:>8.2} μs ({:.3} ms)", client_avg, client_avg / 1000.0);
    println!("     50th Percentile    : {:>8} μs ({:.3} ms)", client_p50, client_p50 as f64 / 1000.0);
    println!("     90th Percentile    : {:>8} μs ({:.3} ms)", client_p90, client_p90 as f64 / 1000.0);
    println!("     95th Percentile    : {:>8} μs ({:.3} ms)", client_p95, client_p95 as f64 / 1000.0);
    println!("     99th Percentile    : {:>8} μs ({:.3} ms)", client_p99, client_p99 as f64 / 1000.0);
    println!("     99.9th Percentile  : {:>8} μs ({:.3} ms)", client_p999, client_p999 as f64 / 1000.0);
    println!("     Max Latency        : {:>8} μs ({:.3} ms)", client_max, client_max as f64 / 1000.0);

    if !server_latencies.is_empty() {
        let srv_sum: u64 = server_latencies.iter().sum();
        let srv_avg = (srv_sum as f64) / (server_latencies.len() as f64);
        let srv_min = *server_latencies.first().unwrap_or(&0);
        let srv_max = *server_latencies.last().unwrap_or(&0);
        let srv_p50 = calc_percentile(&server_latencies, 50.0);
        let srv_p90 = calc_percentile(&server_latencies, 90.0);
        let srv_p99 = calc_percentile(&server_latencies, 99.0);

        println!("--------------------------------------------------------------");
        println!("   INTERNAL SERVER EXECUTION TIME (Reported via X-Response-Time):");
        println!("     Min Server Time    : {:>8} μs ({:.3} ms)", srv_min, srv_min as f64 / 1000.0);
        println!("     Mean Server Time   : {:>8.2} μs ({:.3} ms)", srv_avg, srv_avg / 1000.0);
        println!("     50th Percentile    : {:>8} μs ({:.3} ms)", srv_p50, srv_p50 as f64 / 1000.0);
        println!("     90th Percentile    : {:>8} μs ({:.3} ms)", srv_p90, srv_p90 as f64 / 1000.0);
        println!("     99th Percentile    : {:>8} μs ({:.3} ms)", srv_p99, srv_p99 as f64 / 1000.0);
        println!("     Max Server Time    : {:>8} μs ({:.3} ms)", srv_max, srv_max as f64 / 1000.0);
    }
    println!("==============================================================");
}
