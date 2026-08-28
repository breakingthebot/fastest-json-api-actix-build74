//! tests/cache_service_tests.rs
//! Unit tests for the 64-way sharded cache service and TTL engine.
//! Connects to: src/services/cache_service.rs, src/models/cache.rs
//! Created: 2026-08-28

use fastest_json_api_actix::models::BatchCacheItem;
use fastest_json_api_actix::services::cache_service::{ShardedCacheService, NUM_SHARDS};
use serde_json::json;
use std::thread;
use std::time::Duration;

#[test]
fn test_cache_set_get_and_hit_tracking() {
    let cache = ShardedCacheService::new();

    let shard_id = cache.set("user:1001".to_string(), json!({"name": "Alice", "role": "admin"}), None);
    assert!(shard_id < NUM_SHARDS);

    // First get -> hit #1
    let item1 = cache.get("user:1001").expect("Expected cache hit");
    assert_eq!(item1.key, "user:1001");
    assert_eq!(item1.value["name"], "Alice");
    assert_eq!(item1.hits, 1);
    assert_eq!(item1.ttl_remaining_ms, None);

    // Second get -> hit #2
    let item2 = cache.get("user:1001").expect("Expected cache hit");
    assert_eq!(item2.hits, 2);

    let stats = cache.get_stats();
    assert_eq!(stats.total_gets, 2);
    assert_eq!(stats.cache_hits, 2);
    assert_eq!(stats.cache_misses, 0);
    assert_eq!(stats.hit_ratio_pct, 100.0);
}

#[test]
fn test_cache_ttl_expiration_and_purge() {
    let cache = ShardedCacheService::new();

    // Set item with 1 second TTL
    cache.set("session:temp".to_string(), json!({"token": "xyz123"}), Some(1));

    // Immediate get should hit
    assert!(cache.get("session:temp").is_some());

    // Wait 1.1 seconds for expiration
    thread::sleep(Duration::from_millis(1100));

    // Get after expiration should miss and auto-evict
    assert!(cache.get("session:temp").is_none());

    let stats = cache.get_stats();
    assert_eq!(stats.cache_misses, 1);
    assert_eq!(stats.total_expired_evictions, 1);
}

#[test]
fn test_cache_batch_set_and_delete() {
    let cache = ShardedCacheService::new();

    let items = vec![
        BatchCacheItem {
            key: "item:1".to_string(),
            value: json!({"price": 10}),
            ttl_seconds: None,
        },
        BatchCacheItem {
            key: "item:2".to_string(),
            value: json!({"price": 20}),
            ttl_seconds: None,
        },
        BatchCacheItem {
            key: "item:3".to_string(),
            value: json!({"price": 30}),
            ttl_seconds: None,
        },
    ];

    let count = cache.batch_set(items);
    assert_eq!(count, 3);

    assert_eq!(cache.get("item:1").unwrap().value["price"], 10);
    assert_eq!(cache.get("item:2").unwrap().value["price"], 20);
    assert_eq!(cache.get("item:3").unwrap().value["price"], 30);

    let deleted = cache.delete("item:2");
    assert!(deleted);
    assert!(cache.get("item:2").is_none());

    cache.clear();
    assert_eq!(cache.get_stats().total_keys, 0);
}
