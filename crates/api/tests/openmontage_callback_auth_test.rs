//! OpenMontage Internal Callback Auth Tests — Bug B01 (P0 security)
//!
//! INTENTIONALLY RED pending the B01 fix. The internal callback endpoint
//! `POST /internal/openmontage/callback` currently has NO authentication: the
//! route doc-comment and handler CLAIM an `X-Internal-Token` check, but none is
//! implemented in `routes/openmontage.rs::internal_routes`,
//! `routes/root.rs` (the `/internal/openmontage` nest), or
//! `handler/openmontage_handler.rs::ingest_callback`. Any client can POST
//! arbitrary job events. These tests encode the correct behavior the fixer will
//! implement; they MUST stay RED until B01 is fixed.
//!
//! ============================================================================
//! INTERFACE CONTRACT (the fixer implements this EXACTLY)
//! ============================================================================
//! The internal callback MUST require a valid `X-Internal-Token` header whose
//! value equals the `OPENMONTAGE_INTERNAL_TOKEN` environment variable.
//!
//!   - Token source: the `OPENMONTAGE_INTERNAL_TOKEN` env var.
//!   - Header name:  `X-Internal-Token` (case-insensitive per HTTP).
//!   - Missing header  ⇒ 401 (or 403).
//!   - Wrong token     ⇒ 401 (or 403).
//!   - Correct token   ⇒ 200 (processes the event exactly as today).
//!
//! IMPLEMENTATION GUIDANCE FOR THE FIXER (pick the simplest):
//!   Add an axum middleware (`middleware::from_fn`, applied to
//!   `internal_routes()` in `routes/openmontage.rs` and/or layered at the
//!   `/internal/openmontage` nest in `routes/root.rs`) that:
//!     1. reads `std::env::var("OPENMONTAGE_INTERNAL_TOKEN")`,
//!     2. reads the `X-Internal-Token` request header,
//!     3. returns `StatusCode::UNAUTHORIZED` (401) when the header is missing
//!        or does not equal the env var,
//!     4. otherwise calls `next.run(req)` so the handler runs unchanged.
//!   The middleware should return a RAW `StatusCode` (not via `ApiError`, which
//!   is mapped through the unified `ApiResponse` and would not surface a 401/403
//!   HTTP status). These tests assert on the raw HTTP status code.
//!
//! This test builds the internal router the same way `openmontage_callback_test.rs`
//! does — `internal_routes()` with an injected `Extension<OpenMontageService>`,
//! driven via `tower::ServiceExt::oneshot` — and sets `OPENMONTAGE_INTERNAL_TOKEN`
//! via `std::env::set_var` so the middleware can read it.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::{
    repository::openmontage_repository::{InMemoryJobStore, NewJob, OpenMontageJobStore},
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// The token value used across these tests. Set into `OPENMONTAGE_INTERNAL_TOKEN`
/// so the (to-be-implemented) middleware reads it.
const TEST_TOKEN: &str = "super-secret-internal-token";

/// The `job_id` in `callback_event_body()` — must match the seeded job below so
/// that a CORRECTLY authenticated callback finds its job and returns 200 (rather
/// than a "Job not found" 500). This guarantees the correct-token case is GREEN
/// purely because of valid auth, and the no-token/wrong-token cases are RED
/// purely because of MISSING auth (not because of any unrelated 500).
const FIXTURE_JOB_ID: &str = "job-fixture-1";

/// Build the internal callback router exactly like `openmontage_callback_test.rs`:
/// `internal_routes()` with an injected `Extension<OpenMontageService>` and no
/// user auth. The job referenced by the callback body is pre-seeded so that an
/// authenticated callback processes successfully (200). Once B01 is fixed, this
/// router will carry the X-Internal-Token middleware.
fn test_callback_router() -> axum::Router {
    use glance_mind_api::routes::openmontage;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());

    // Seed the job the callback event targets so a well-formed, authenticated
    // callback succeeds with 200 (mirrors the seeding pattern in
    // `openmontage_facade_test.rs`).
    let new_job = NewJob {
        job_id: FIXTURE_JOB_ID.to_string(),
        project_id: "proj-fixture-1".to_string(),
        user_id: 1,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-fixture-1".to_string(),
        idempotency_key: "idem-fixture-1".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text".to_string()),
        status: "running".to_string(),
        snapshot_json: serde_json::json!({"title": "Auth Test Job"}),
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    openmontage::internal_routes().layer(axum::Extension(service))
}

/// A valid worker-emitted callback event body. Mirrors the fixture used by
/// `openmontage_callback_test.rs:74`
/// (`worker_event_job_completed.json`); falls back to an inline copy if the
/// cross-worktree fixture path is unavailable, so the test is hermetic.
fn callback_event_body() -> String {
    let fixture_path = "/Users/jacksoom/programer/aihub/.worktrees/omx-product-api/glance_mind_worker/openmontage_executor/tests/fixtures/worker_event_job_completed.json";

    match std::fs::read_to_string(fixture_path) {
        Ok(content) => content,
        Err(_) => json!({
            "version": "v1",
            "event_id": "550e8400-e29b-41d4-a716-446655440000",
            "sequence": 42,
            "job": {
                "job_id": "job-fixture-1",
                "request_id": "req-fixture-1",
                "project_id": "proj-fixture-1",
                "correlation_id": "",
                "idempotency_key": "idem-fixture-1"
            },
            "event_type": "job_completed",
            "status": "completed",
            "stage": "",
            "progress_pct": 100,
            "emitted_at": "2024-01-01T12:00:00.000000Z",
            "artifacts": [
                {
                    "artifact_id": "artifact-final-video",
                    "kind": "video",
                    "role": "primary_video",
                    "uri": "",
                    "path": "/tmp/openmontage_projects/proj-fixture-1/renders/final.mp4",
                    "mime_type": "video/mp4",
                    "width_px": 320,
                    "height_px": 240,
                    "duration_ms": 1000,
                    "bytes": 12345
                }
            ]
        })
        .to_string(),
    }
}

/// B01: a callback with NO `X-Internal-Token` header MUST be rejected (401/403).
///
/// RED today: no auth is implemented, so the handler runs and returns 200.
#[tokio::test]
async fn callback_without_token_is_rejected() {
    std::env::set_var("OPENMONTAGE_INTERNAL_TOKEN", TEST_TOKEN);

    let router = test_callback_router();

    let req = Request::builder()
        .method("POST")
        // `internal_routes()` mounts the route at its relative path `/callback`
        // (the `/internal/openmontage` prefix is added by the `.nest(...)` in
        // `routes/root.rs`, which is not exercised by this router-level test).
        .uri("/callback")
        .header("content-type", "application/json")
        // No X-Internal-Token header at all.
        .body(Body::from(callback_event_body()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    let status = response.status();

    assert!(
        status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
        "missing X-Internal-Token must be rejected with 401 or 403, was {}",
        status
    );
}

/// B01: a callback with a WRONG `X-Internal-Token` value MUST be rejected (401/403).
///
/// RED today: no auth is implemented, so the handler runs and returns 200.
#[tokio::test]
async fn callback_with_wrong_token_is_rejected() {
    std::env::set_var("OPENMONTAGE_INTERNAL_TOKEN", TEST_TOKEN);

    let router = test_callback_router();

    let req = Request::builder()
        .method("POST")
        // `internal_routes()` mounts the route at its relative path `/callback`
        // (the `/internal/openmontage` prefix is added by the `.nest(...)` in
        // `routes/root.rs`, which is not exercised by this router-level test).
        .uri("/callback")
        .header("content-type", "application/json")
        .header("x-internal-token", "totally-wrong-token")
        .body(Body::from(callback_event_body()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    let status = response.status();

    assert!(
        status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
        "wrong X-Internal-Token must be rejected with 401 or 403, was {}",
        status
    );
}

/// B01: a callback with the CORRECT `X-Internal-Token` value MUST be accepted (200)
/// and processed exactly as today.
///
/// This case passes today (no auth ⇒ handler always runs) and must KEEP passing
/// after the fix — it guards against the fixer over-rejecting valid callbacks.
#[tokio::test]
async fn callback_with_correct_token_is_accepted() {
    std::env::set_var("OPENMONTAGE_INTERNAL_TOKEN", TEST_TOKEN);

    let router = test_callback_router();

    let req = Request::builder()
        .method("POST")
        // `internal_routes()` mounts the route at its relative path `/callback`
        // (the `/internal/openmontage` prefix is added by the `.nest(...)` in
        // `routes/root.rs`, which is not exercised by this router-level test).
        .uri("/callback")
        .header("content-type", "application/json")
        .header("x-internal-token", TEST_TOKEN)
        .body(Body::from(callback_event_body()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    let status = response.status();

    assert_eq!(
        status,
        StatusCode::OK,
        "correct X-Internal-Token must be accepted with 200, was {}",
        status
    );
}
