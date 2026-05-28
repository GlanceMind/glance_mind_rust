//! OpenMontage Routes
//!
//! User routes (E1-E8) require JWT auth.
//! Internal routes (I1) use X-Internal-Token header auth.

use axum::{
    routing::{get, post},
    Router,
};

use crate::handler::openmontage_handler;

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
}

/// Internal routes (mounted under /api/v1/internal/openmontage)
pub fn internal_routes() -> Router {
    Router::new().route("/callback", post(openmontage_handler::ingest_callback))
}
