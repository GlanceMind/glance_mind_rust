//! Internal-service authentication middleware (B01).
//!
//! Guards internal-only endpoints (e.g. the OpenMontage worker callback
//! `POST /internal/openmontage/callback`) by requiring an `X-Internal-Token`
//! request header whose value equals the `OPENMONTAGE_INTERNAL_TOKEN`
//! environment variable.
//!
//! Behavior (fail-closed):
//!   - `OPENMONTAGE_INTERNAL_TOKEN` unset or empty ⇒ reject ALL requests (401).
//!   - `X-Internal-Token` missing                  ⇒ reject (401).
//!   - `X-Internal-Token` != configured token      ⇒ reject (401).
//!   - `X-Internal-Token` == configured token      ⇒ pass through.
//!
//! Returns a RAW `StatusCode::UNAUTHORIZED` on rejection (not via `ApiError` /
//! the unified `ApiResponse`, which would remap to a 200-with-error-body) so
//! that callers see a genuine HTTP 401.

use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

/// Environment variable holding the shared secret for internal callbacks.
const INTERNAL_TOKEN_ENV: &str = "OPENMONTAGE_INTERNAL_TOKEN";

/// Request header carrying the internal token.
const INTERNAL_TOKEN_HEADER: &str = "x-internal-token";

/// Axum middleware that enforces the `X-Internal-Token` shared secret.
pub async fn require_internal_token<B>(req: Request<B>, next: Next<B>) -> Response {
    // Fail-closed: if the secret is unset or empty, reject everything.
    let expected = match std::env::var(INTERNAL_TOKEN_ENV) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            tracing::warn!(
                "Rejecting internal callback: {} is unset or empty (fail-closed)",
                INTERNAL_TOKEN_ENV
            );
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let presented = req
        .headers()
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok());

    match presented {
        Some(token) if constant_time_eq(token.as_bytes(), expected.as_bytes()) => {
            next.run(req).await
        }
        _ => {
            tracing::warn!("Rejecting internal callback: missing or invalid X-Internal-Token");
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

/// Constant-time byte comparison (wraps `subtle::ConstantTimeEq`).
///
/// Pads the comparison on length mismatch to avoid length-timing leaks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        let dummy = vec![0u8; b.len()];
        let _ = dummy.ct_eq(b);
        return false;
    }
    a.ct_eq(b).into()
}
