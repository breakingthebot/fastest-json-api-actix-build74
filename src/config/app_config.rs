//! src/config/app_config.rs
//! Application runtime configuration loaded from environment variables.
//! Connects to: src/main.rs, src/config/mod.rs
//! Created: 2026-08-27

use std::env;

/// Application runtime configuration parameters.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Server host IP or domain to bind (e.g. 127.0.0.1 or 0.0.0.0)
    pub host: String,
    /// TCP Port to listen on (default: 8080)
    pub port: u16,
    /// Number of worker threads for the Actix actor system (default: number of logical CPUs)
    pub workers: usize,
    /// TCP keep-alive duration in seconds (default: 75)
    pub keep_alive_secs: u64,
    /// Maximum pending TCP connections backlog size (default: 2048)
    pub backlog: i32,
    /// Maximum JSON payload size in bytes (default: 2MB = 2,097,152 bytes)
    pub max_payload_bytes: usize,
    /// Application environment name (development, staging, production)
    pub environment: String,
}

impl AppConfig {
    /// Loads configuration from environment variables or applies high-performance defaults.
    ///
    /// # Returns
    /// An initialized `AppConfig` instance.
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("SERVER_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);
        let workers = env::var("SERVER_WORKERS")
            .ok()
            .and_then(|w| w.parse::<usize>().ok())
            .unwrap_or_else(num_cpus::get);
        let keep_alive_secs = env::var("SERVER_KEEPALIVE_SECS")
            .ok()
            .and_then(|k| k.parse::<u64>().ok())
            .unwrap_or(75);
        let backlog = env::var("SERVER_BACKLOG")
            .ok()
            .and_then(|b| b.parse::<i32>().ok())
            .unwrap_or(2048);
        let max_payload_bytes = env::var("MAX_PAYLOAD_BYTES")
            .ok()
            .and_then(|m| m.parse::<usize>().ok())
            .unwrap_or(2 * 1024 * 1024);
        let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());

        Self {
            host,
            port,
            workers,
            keep_alive_secs,
            backlog,
            max_payload_bytes,
            environment,
        }
    }

    /// Formats the socket binding address string.
    ///
    /// # Returns
    /// Socket address string formatted as `host:port`.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
