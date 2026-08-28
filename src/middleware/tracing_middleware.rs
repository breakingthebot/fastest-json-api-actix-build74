//! src/middleware/tracing_middleware.rs
//! W3C Trace Context and distributed tracing header propagation middleware.
//! Connects to: src/models/tracing.rs, src/main.rs
//! Created: 2026-08-28

use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{Error, HttpMessage};

use crate::models::TraceContext;

/// Middleware transform for W3C distributed tracing context propagation.
#[derive(Clone)]
pub struct TracingMiddleware;

impl<S, B> Transform<S, ServiceRequest> for TracingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TracingInnerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(TracingInnerMiddleware { service }))
    }
}

pub struct TracingInnerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TracingInnerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let headers = req.headers();

        // 1. Parse or generate W3C traceparent header
        let (trace_id, parent_span_id, trace_flags) = if let Some(tp_val) = headers
            .get("traceparent")
            .and_then(|h| h.to_str().ok())
        {
            let parts: Vec<&str> = tp_val.split('-').collect();
            if parts.len() == 4 && parts[0] == "00" && parts[1].len() == 32 && parts[2].len() == 16 {
                (parts[1].to_string(), Some(parts[2].to_string()), parts[3].to_string())
            } else {
                (generate_trace_id(), None, "01".to_string())
            }
        } else {
            (generate_trace_id(), None, "01".to_string())
        };

        let span_id = generate_span_id();
        let tracestate = headers
            .get("tracestate")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let is_sampled = trace_flags == "01";

        let trace_context = TraceContext {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            parent_span_id,
            trace_flags: trace_flags.clone(),
            tracestate: tracestate.clone(),
            is_sampled,
        };

        // Attach context to request extensions for handler access
        req.extensions_mut().insert(trace_context.clone());

        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let res_headers = res.headers_mut();

            // Inject W3C Traceparent header into response
            let traceparent_str = format!("00-{}-{}-{}", trace_id, span_id, trace_flags);
            if let Ok(val) = HeaderValue::from_str(&traceparent_str) {
                res_headers.insert(HeaderName::from_static("traceparent"), val);
            }

            // Inject convenience headers
            if let Ok(val) = HeaderValue::from_str(&trace_id) {
                res_headers.insert(HeaderName::from_static("x-trace-id"), val);
            }

            if let Ok(val) = HeaderValue::from_str(&span_id) {
                res_headers.insert(HeaderName::from_static("x-span-id"), val);
            }

            if let Some(ref ts) = tracestate {
                if let Ok(val) = HeaderValue::from_str(ts) {
                    res_headers.insert(HeaderName::from_static("tracestate"), val);
                }
            }

            Ok(res)
        })
    }
}

/// Generates a 32-character hex trace ID using nanosecond entropy.
fn generate_trace_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let rand_val = (now ^ (now >> 32)) as u64;
    format!("{:016x}{:016x}", now as u64, rand_val)
}

/// Generates a 16-character hex span ID.
fn generate_span_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}", (now ^ (now >> 64)) as u64)
}
