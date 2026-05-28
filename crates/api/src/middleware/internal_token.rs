use crate::error::api_error::ApiError;
use axum::{
    body::BoxBody,
    http::{Request, Response},
    middleware::Next,
    response::IntoResponse,
};
use std::env;

pub const HEADER_NAME: &str = "X-Internal-Token";
pub const ENV_VAR: &str = "INTERNAL_SERVICE_TOKEN";

/// Require a matching `X-Internal-Token` header against
/// `INTERNAL_SERVICE_TOKEN` env var. If the env var is unset or empty the
/// middleware rejects all requests (fail-closed) so misconfiguration cannot
/// accidentally expose the internal surface.
///
/// Rejections are surfaced through `ApiError::Unauthorized` so that they
/// serialize with the canonical `ApiResponse { code, msg, msg_cn, data }`
/// shape used by the rest of the service (unified error code 2002).
pub async fn require_internal_token<B>(req: Request<B>, next: Next<B>) -> Response<BoxBody> {
    let expected = env::var(ENV_VAR).unwrap_or_default();
    if expected.is_empty() {
        return ApiError::Unauthorized("internal auth not configured".into()).into_response();
    }

    let header = req.headers().get(HEADER_NAME).and_then(|v| v.to_str().ok());
    match header {
        Some(got) if got == expected => next.run(req).await.into_response(),
        _ => ApiError::Unauthorized("auth_failed".into()).into_response(),
    }
}
