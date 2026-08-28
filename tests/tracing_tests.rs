//! tests/tracing_tests.rs
//! Integration tests for W3C distributed tracing context and header propagation.
//! Connects to: src/middleware/tracing_middleware.rs, src/handlers/trace.rs
//! Created: 2026-08-28

use actix_web::{test, web, App};
use fastest_json_api_actix::handlers::configure_routes;
use fastest_json_api_actix::middleware::{LatencyTracker, TracingMiddleware};
use fastest_json_api_actix::models::TraceInspectionResponse;
use fastest_json_api_actix::services::{MetricsService, RingBufferService, ShardedCacheService};
use std::sync::Arc;

#[actix_web::test]
async fn test_auto_generated_trace_headers() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    // Send request without traceparent header
    let req = test::TestRequest::get().uri("/api/v1/ping").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status().as_u16(), 200);

    let headers = resp.headers();
    assert!(headers.contains_key("traceparent"));
    assert!(headers.contains_key("x-trace-id"));
    assert!(headers.contains_key("x-span-id"));

    let tp = headers.get("traceparent").unwrap().to_str().unwrap();
    assert!(tp.starts_with("00-"));
    let parts: Vec<&str> = tp.split('-').collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[1].len(), 32); // 16-byte trace id
    assert_eq!(parts[2].len(), 16); // 8-byte span id
}

#[actix_web::test]
async fn test_w3c_traceparent_propagation() {
    let metrics_service = Arc::new(MetricsService::new());
    let ring_buffer_service = Arc::new(RingBufferService::new());
    let cache_service = Arc::new(ShardedCacheService::new());

    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware)
            .wrap(LatencyTracker)
            .app_data(web::Data::new(metrics_service))
            .app_data(web::Data::new(ring_buffer_service))
            .app_data(web::Data::new(cache_service))
            .configure(configure_routes),
    )
    .await;

    let incoming_trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let incoming_parent_span = "00f067aa0ba902b7";
    let traceparent_header = format!("00-{}-{}-01", incoming_trace_id, incoming_parent_span);

    let req = test::TestRequest::get()
        .uri("/api/v1/trace/current")
        .insert_header(("traceparent", traceparent_header.as_str()))
        .insert_header(("tracestate", "congo=t61rcWkgMzE,rojo=00f067aa0ba902b7"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let resp_headers = resp.headers();
    assert_eq!(
        resp_headers.get("x-trace-id").unwrap().to_str().unwrap(),
        incoming_trace_id
    );
    assert_eq!(
        resp_headers.get("tracestate").unwrap().to_str().unwrap(),
        "congo=t61rcWkgMzE,rojo=00f067aa0ba902b7"
    );

    let body: TraceInspectionResponse = test::read_body_json(resp).await;
    assert_eq!(body.context.trace_id, incoming_trace_id);
    assert_eq!(body.context.parent_span_id, Some(incoming_parent_span.to_string()));
    assert!(body.context.is_sampled);
}
