//! src/handlers/benchmark.rs
//! High-throughput synthetic payload generation and batch ingestion handlers.
//! Connects to: src/models/benchmark.rs, src/handlers/mod.rs
//! Created: 2026-08-27

use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use std::time::Instant;

use crate::models::{BenchmarkItem, BenchmarkResponse, IngestRequest, IngestResponse};

/// Query parameters for synthetic benchmark data generation.
#[derive(Debug, Deserialize)]
pub struct BenchmarkQuery {
    /// Named size category: 'small' (10 items), 'medium' (100 items), 'large' (1000 items), 'xlarge' (5000 items)
    pub size: Option<String>,
    /// Explicit item count (overrides size, clamped to 1..=10,000)
    pub count: Option<usize>,
}

/// Handler for `GET /api/v1/benchmark/synthetic`.
///
/// # Arguments
/// * `query` - Query parameters specifying batch size or item count
///
/// # Returns
/// HTTP 200 OK with `BenchmarkResponse` containing generated items.
pub async fn get_synthetic_data(query: web::Query<BenchmarkQuery>) -> impl Responder {
    let (item_count, category) = if let Some(explicit_count) = query.count {
        let count = explicit_count.clamp(1, 10_000);
        (count, "custom".to_string())
    } else {
        match query.size.as_deref().unwrap_or("small") {
            "medium" => (100, "medium".to_string()),
            "large" => (1_000, "large".to_string()),
            "xlarge" => (5_000, "xlarge".to_string()),
            _ => (10, "small".to_string()),
        }
    };

    let mut items = Vec::with_capacity(item_count);
    for i in 1..=(item_count as u64) {
        items.push(BenchmarkItem::generate(i));
    }

    // Estimate JSON payload bytes
    let estimated_bytes = items.len() * 155;

    let response = BenchmarkResponse {
        generated_at: chrono::Utc::now().to_rfc3339(),
        size_category: category,
        item_count,
        estimated_bytes,
        items,
    };

    HttpResponse::Ok().json(response)
}

/// Handler for `POST /api/v1/benchmark/ingest`.
///
/// # Arguments
/// * `payload` - Ingest batch JSON body
///
/// # Returns
/// HTTP 200 OK with `IngestResponse` summarizing items processed and total price sum.
pub async fn post_ingest_data(payload: web::Json<IngestRequest>) -> impl Responder {
    let start_time = Instant::now();
    let batch = payload.into_inner();

    let items_count = batch.items.len();
    let total_value_cents: u64 = batch
        .items
        .iter()
        .map(|item| (item.price_cents as u64) * (item.stock_level.max(0) as u64))
        .sum();

    let duration_us = start_time.elapsed().as_micros() as u64;

    let response = IngestResponse {
        status: "success".to_string(),
        batch_id: batch.batch_id,
        items_processed: items_count,
        total_value_cents,
        duration_us,
    };

    HttpResponse::Ok().json(response)
}
