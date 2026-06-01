//! API error taxonomy.
//!
//! Every fallible handler returns [`ApiError`]. The `IntoResponse` impl maps
//! each variant to a stable HTTP status + a JSON body of the shape
//! `{ "error": "...", "kind": "..." }`. The `kind` field is a machine-stable
//! discriminant so clients can branch without string-matching the message.
//!
//! Status mapping:
//!
//! | Variant        | Status | When |
//! |----------------|--------|------|
//! | `BadRequest`   | 400    | Malformed input / failed domain validation |
//! | `PayloadTooLarge` | 413 | Body exceeded the configured limit |
//! | `RateLimited`  | 429    | Per-tenant token bucket exhausted |
//! | `Overloaded`   | 503    | Concurrency limit reached (load shed) |
//! | `Timeout`      | 504    | Request exceeded the hard time ceiling |
//! | `Internal`     | 500    | Unexpected server fault |

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Error type returned by Tyche API handlers.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Malformed request or failed domain validation.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// The request body exceeded the configured size limit.
    #[error("payload too large")]
    PayloadTooLarge,

    /// The caller exceeded its rate limit. Carries the retry-after hint (s).
    #[error("rate limited; retry after {0}s")]
    RateLimited(u64),

    /// The server is shedding load due to a concurrency limit.
    #[error("overloaded")]
    Overloaded,

    /// The request exceeded the hard time ceiling.
    #[error("request timed out")]
    Timeout,

    /// An unexpected internal fault.
    #[error("internal error: {0}")]
    Internal(String),
}

/// JSON body emitted for every error.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    kind: &'static str,
}

impl ApiError {
    const fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    const fn kind(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "bad_request",
            Self::PayloadTooLarge => "payload_too_large",
            Self::RateLimited(_) => "rate_limited",
            Self::Overloaded => "overloaded",
            Self::Timeout => "timeout",
            Self::Internal(_) => "internal",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.kind();
        let retry_after = match &self {
            Self::RateLimited(s) => Some(*s),
            Self::Overloaded => Some(1),
            _ => None,
        };
        let body = Json(ErrorBody {
            error: self.to_string(),
            kind,
        });
        match retry_after {
            Some(secs) => (status, [(header::RETRY_AFTER, secs.to_string())], body).into_response(),
            None => (status, body).into_response(),
        }
    }
}
