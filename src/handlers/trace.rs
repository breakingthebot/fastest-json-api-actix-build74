//! src/handlers/trace.rs
//! Distributed trace inspection endpoint handler.
//! Connects to: src/models/tracing.rs, src/handlers/mod.rs
//! Created: 2026-08-28

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder};

use crate::models::{TraceContext, TraceInspectionResponse};

/// Handler for `GET /api/v1/trace/current`.
///
/// # Arguments
/// * `req` - Inbound HTTP request reference
///
/// # Returns
/// HTTP 200 OK with `TraceInspectionResponse` JSON.
pub async fn get_current_trace(req: HttpRequest) -> impl Responder {
    let context = req
        .extensions()
        .get::<TraceContext>()
        .cloned()
        .unwrap_or_else(|| TraceContext {
            trace_id: "unknown".to_string(),
            span_id: "unknown".to_string(),
            parent_span_id: None,
            trace_flags: "00".to_string(),
            tracestate: None,
            is_sampled: false,
        });

    let traceparent = context.to_traceparent();
    let tracestate = context.tracestate.clone();

    let response = TraceInspectionResponse {
        context,
        traceparent,
        tracestate,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    HttpResponse::Ok().json(response)
}
