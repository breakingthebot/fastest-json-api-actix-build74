//! src/middleware/rate_limit_middleware.rs
//! High-throughput Token Bucket rate limiting middleware with standard RFC headers and 429 responses.
//! Connects to: src/services/rate_limiter.rs, src/models/rate_limit.rs, src/main.rs
//! Created: 2026-08-28

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::{web, Error, HttpResponse};
use std::future::{ready, Future, Ready};
use std::pin::Pin;
use std::sync::Arc;

use crate::models::RateLimitErrorResponse;
use crate::services::RateLimiterService;

/// Middleware transform for Token Bucket rate limiting.
#[derive(Clone)]
pub struct RateLimitMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitInnerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitInnerMiddleware { service }))
    }
}

pub struct RateLimitInnerMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RateLimitInnerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();

        // 1. Bypass rate limiting for heartbeat, health, admin stats, and observability endpoints
        if path == "/ping"
            || path == "/health"
            || path == "/metrics"
            || path == "/dashboard"
            || path == "/ws/metrics"
            || path == "/api/v1/ping"
            || path == "/api/v1/health"
            || path == "/api/v1/metrics"
            || path == "/api/v1/metrics/prometheus"
            || path == "/api/v1/stream/metrics"
            || path == "/api/v1/stream/dashboard"
            || path == "/api/v1/ratelimit/stats"
            || path == "/api/v1/ratelimit/reset"
            || path == "/api/v1/wal/stats"
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // 2. Extract rate limiter service from app data
        let rate_limiter = req.app_data::<web::Data<Arc<RateLimiterService>>>().cloned();

        let client_key = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                req.headers()
                    .get("x-real-ip")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                req.peer_addr()
                    .map(|addr| addr.ip().to_string())
            })
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let decision = if let Some(ref limiter) = rate_limiter {
            limiter.try_acquire(&client_key)
        } else {
            crate::models::RateLimitDecision {
                allowed: true,
                limit: 1000,
                remaining: 1000,
                reset_seconds: 1,
            }
        };

        // 3. If rejected -> return HTTP 429 Too Many Requests immediately
        if !decision.allowed {
            let error_payload = RateLimitErrorResponse {
                status: StatusCode::TOO_MANY_REQUESTS.as_u16(),
                error: "Too Many Requests".to_string(),
                message: format!(
                    "Rate limit of {} burst capacity exceeded. Please retry in {} seconds.",
                    decision.limit, decision.reset_seconds
                ),
                retry_after_seconds: decision.reset_seconds,
                instance: path,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let mut http_resp = HttpResponse::build(StatusCode::TOO_MANY_REQUESTS)
                .json(error_payload);

            let res_headers = http_resp.headers_mut();
            let _ = HeaderValue::from_str(&decision.limit.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-limit"), v));
            let _ = HeaderValue::from_str(&decision.remaining.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-remaining"), v));
            let _ = HeaderValue::from_str(&decision.reset_seconds.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-reset"), v));
            let _ = HeaderValue::from_str(&decision.reset_seconds.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("retry-after"), v));

            let (req_inner, _) = req.into_parts();
            let srv_resp = ServiceResponse::new(req_inner, http_resp.map_into_right_body());
            return Box::pin(async move { Ok(srv_resp) });
        }

        // 4. Request allowed -> execute pipeline and inject rate limit headers into response
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let res_headers = res.headers_mut();

            let _ = HeaderValue::from_str(&decision.limit.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-limit"), v));
            let _ = HeaderValue::from_str(&decision.remaining.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-remaining"), v));
            let _ = HeaderValue::from_str(&decision.reset_seconds.to_string())
                .map(|v| res_headers.insert(HeaderName::from_static("x-ratelimit-reset"), v));

            Ok(res.map_into_left_body())
        })
    }
}
