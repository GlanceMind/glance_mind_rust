//! OpenMontage Routes
//!
//! User routes (E1-E8) require JWT auth.
//! Internal routes (I1) use X-Internal-Token header auth.

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::handler::openmontage_handler;
use crate::middleware::internal_auth;

/// User-facing routes (mounted under /api/v1/openmontage)
pub fn user_routes() -> Router {
    Router::new()
        .route("/preflight", get(openmontage_handler::get_preflight))
        .route("/pipelines", get(openmontage_handler::get_pipelines))
        .route("/jobs", post(openmontage_handler::create_job))
        .route("/jobs/:job_id", get(openmontage_handler::get_job))
        .route("/jobs/:job_id/events", get(openmontage_handler::get_events))
        .route("/jobs/:job_id/stream", get(openmontage_handler::stream_job))
        .route(
            "/jobs/:job_id/approvals",
            post(openmontage_handler::submit_approval),
        )
        .route(
            "/jobs/:job_id/cancel",
            post(openmontage_handler::cancel_job),
        )
        .route("/assets", post(openmontage_handler::upload_asset))
}

/// Internal routes (mounted under /api/v1/internal/openmontage)
///
/// Guarded by the `X-Internal-Token` shared-secret middleware (B01): the token
/// must equal the `OPENMONTAGE_INTERNAL_TOKEN` env var, else the request is
/// rejected with a raw `401 Unauthorized`. Applied here (rather than at the
/// `/internal/openmontage` nest) so every mount of `internal_routes()` is
/// guarded.
pub fn internal_routes() -> Router {
    Router::new()
        .route("/callback", post(openmontage_handler::ingest_callback))
        .layer(middleware::from_fn(internal_auth::require_internal_token))
}
