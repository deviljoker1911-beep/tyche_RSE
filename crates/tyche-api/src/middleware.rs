//! Custom middleware: per-tenant rate limiting and HTTP metric recording.

use std::time::Instant;

use axum::extract::{MatchedPath, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;
use crate::metrics::names;
use crate::rate_limit::Decision;
use crate::state::AppState;

/// Header carrying the tenant identity used for rate-limit bucketing.
///
/// In production this is set by the authenticating gateway from the verified
/// SSO identity; a client cannot forge it because the gateway overwrites any
/// inbound value. In the spike it is trusted as-is.
pub const TENANT_HEADER: &str = "x-tyche-tenant";

/// Per-tenant token-bucket rate limit. Denies with 429 + `Retry-After`.
pub async fn rate_limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let key = req
        .headers()
        .get(TENANT_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();

    match state.limiter.check(&key) {
        Decision::Allow => next.run(req).await,
        Decision::Deny { retry_after_secs } => {
            ApiError::RateLimited(retry_after_secs).into_response()
        }
    }
}

/// Record `http_requests_total` and `http_request_duration_seconds` for every
/// request, labelled by method, matched-route template, and final status.
///
/// Uses [`MatchedPath`] (the route *template*, e.g. `/v1/simulate`) rather than
/// the concrete URI, so the cardinality stays bounded regardless of path
/// parameters.
pub async fn track_http_metrics(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().as_str().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |m| m.as_str().to_owned());

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();

    metrics::counter!(
        names::HTTP_REQUESTS,
        "method" => method.clone(),
        "route" => route.clone(),
        "status" => status,
    )
    .increment(1);
    metrics::histogram!(
        names::HTTP_DURATION,
        "method" => method,
        "route" => route,
    )
    .record(elapsed);

    response
}
