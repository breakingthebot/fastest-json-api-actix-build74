//! src/models/benchmark.rs
//! Synthetic data generator and payload models for high-throughput serialization testing.
//! Connects to: src/handlers/benchmark.rs, src/bin/benchmark_client.rs, src/models/mod.rs
//! Created: 2026-08-27

use serde::{Deserialize, Serialize};

/// Synthetic item record representing a realistic e-commerce or telemetry entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkItem {
    /// Unique item sequence identifier
    pub id: u64,
    /// Alphanumeric SKU code
    pub sku: String,
    /// Item display name
    pub name: String,
    /// Category classification
    pub category: String,
    /// Unit price in cents (USD)
    pub price_cents: u32,
    /// Available inventory stock quantity
    pub stock_level: i32,
    /// Active flag
    pub is_active: bool,
    /// List of item attribute tags
    pub tags: Vec<String>,
    /// Nested telemetry payload
    pub telemetry: ItemTelemetry,
}

/// Nested telemetry attributes inside synthetic benchmark item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemTelemetry {
    /// Sensor or warehouse location zone
    pub zone: String,
    /// Temperature reading in Celsius
    pub temperature_c: f64,
    /// Relative humidity percentage
    pub humidity_pct: f64,
    /// Last scanned unix epoch timestamp
    pub last_scanned_epoch: u64,
}

/// Outbound batch container for synthetic benchmark payload responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResponse {
    /// Batch generation timestamp
    pub generated_at: String,
    /// Size classification requested ('small', 'medium', 'large', 'custom')
    pub size_category: String,
    /// Total item count in batch
    pub item_count: usize,
    /// Total calculated JSON payload byte size
    pub estimated_bytes: usize,
    /// Array of synthetic records
    pub items: Vec<BenchmarkItem>,
}

/// Inbound batch ingestion request for throughput measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Batch identifier
    pub batch_id: String,
    /// Client sender label
    pub client_id: String,
    /// Array of items to validate and ingest
    pub items: Vec<BenchmarkItem>,
}

/// Response returned after batch ingestion validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    /// Status message
    pub status: String,
    /// Echoed batch identifier
    pub batch_id: String,
    /// Total items parsed and validated
    pub items_processed: usize,
    /// Total inventory value calculated across the batch in cents
    pub total_value_cents: u64,
    /// Ingestion processing duration in microseconds
    pub duration_us: u64,
}

impl BenchmarkItem {
    /// Generates a deterministic synthetic item based on an index.
    ///
    /// # Arguments
    /// * `index` - Sequential item index for field generation
    ///
    /// # Returns
    /// A fully populated `BenchmarkItem`.
    pub fn generate(index: u64) -> Self {
        let categories = ["Electronics", "Warehouse Supplies", "Sensors", "Hardware", "Packaging"];
        let zones = ["Zone-A", "Zone-B", "Zone-C", "Zone-D", "Zone-E"];
        
        let cat_idx = (index as usize) % categories.len();
        let zone_idx = (index as usize) % zones.len();

        Self {
            id: index,
            sku: format!("SKU-{:08X}", index * 31 + 17),
            name: format!("Precision Actuator Component #{}", index),
            category: categories[cat_idx].to_string(),
            price_cents: ((index % 5000) + 100) as u32,
            stock_level: ((index % 1000) as i32) + 10,
            is_active: index % 7 != 0,
            tags: vec![
                format!("tier-{}", (index % 3) + 1),
                format!("batch-{}", index / 100),
                "actix-benchmark".to_string(),
            ],
            telemetry: ItemTelemetry {
                zone: zones[zone_idx].to_string(),
                temperature_c: 20.0 + ((index % 150) as f64) * 0.1,
                humidity_pct: 40.0 + ((index % 300) as f64) * 0.1,
                last_scanned_epoch: 1724784000 + (index * 60),
            },
        }
    }
}
