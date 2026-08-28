//! src/middleware/latency_tracker.rs
//! Custom Actix-Web middleware measuring sub-millisecond response latency and injecting timing headers.
//! Connects to: src/services/metrics_service.rs, src/main.rs
//! Created: 2026-08-27

use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{web, Error};

use crate::services::MetricsService;

/// Middleware transform factory for tracking latency on all routes.
#[derive(Clone)]
pub struct LatencyTracker;

impl<S, B> Transform<S, ServiceRequest> for LatencyTracker
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LatencyTrackerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LatencyTrackerMiddleware { service }))
    }
}

/// Inner service wrapping each request in microsecond latency measurements.
pub struct LatencyTrackerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LatencyTrackerMiddleware<S>
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
        let start_time = Instant::now();
        let path = req.path().to_string();

        let metrics_service = req.app_data::<web::Data<Arc<MetricsService>>>().cloned();
        if let Some(ref metrics) = metrics_service {
            metrics.record_request_start();
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let elapsed = start_time.elapsed();
            let duration_us = elapsed.as_micros() as u64;
            let duration_ms = (duration_us as f64) / 1000.0;
            let status = res.status().as_u16();

            // Record into atomic metrics
            if let Some(ref metrics) = metrics_service {
                metrics.record_request_completion(status, 0, &path, duration_us);
            }

            // Inject high-resolution performance response headers
            let headers = res.headers_mut();

            if let Ok(val) = HeaderValue::from_str(&duration_us.to_string()) {
                headers.insert(
                    HeaderName::from_static("x-response-time-microseconds"),
                    val,
                );
            }

            if let Ok(val) = HeaderValue::from_str(&format!("{:.3}ms", duration_ms)) {
                headers.insert(
                    HeaderName::from_static("x-response-time-ms"),
                    val,
                );
            }

            if let Ok(val) = HeaderValue::from_str(&format!("total;dur={:.3}", duration_ms)) {
                headers.insert(
                    HeaderName::from_static("x-server-timing"),
                    val,
                );
            }

            headers.insert(
                HeaderName::from_static("server"),
                HeaderValue::from_static("Actix-Rust-UltraFast/0.1.0"),
            );

            Ok(res)
        })
    }
}
