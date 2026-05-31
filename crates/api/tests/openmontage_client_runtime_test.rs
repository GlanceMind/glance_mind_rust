//! Regression: `RedisOpenMontageClient` must not bridge sync -> async via a
//! nested `tokio::runtime::Runtime::new().block_on(...)`.
//!
//! The `OpenMontageClient` trait methods are synchronous, but they are invoked
//! from inside async axum handlers (i.e. on a tokio worker thread). The previous
//! implementation created a *new* multi-thread runtime and called `block_on` on
//! it from within the handler's runtime, which panics at runtime with:
//!
//!     "Cannot start a runtime from within a runtime. This happens because a
//!      function (like `block_on`) attempted to block the current thread while
//!      the thread is being used to drive asynchronous tasks."
//!
//! That panic took down `POST /api/v1/openmontage/jobs` (and every other
//! OpenMontage Redis call) the moment a real `RedisOpenMontageClient` was wired
//! in, surfaced by the live API <-> worker round-trip. The fix uses the redis
//! crate's blocking client directly (the same pattern `lib.rs::init_redis`
//! already uses), with no nested runtime.
//!
//! These tests reproduce the production condition (a multi-thread tokio runtime)
//! and assert each method returns a graceful `Err` against an UNREACHABLE Redis
//! instead of panicking:
//!   * old nested-runtime impl  -> `block_on` panics, the test aborts (RED)
//!   * fixed synchronous impl    -> each call returns `Err(...)`        (GREEN)

use glance_mind_api::service::openmontage_client::{
    OpenMontageClient, RedisOpenMontageClient, WorkerEnvelope,
};

/// A client pointed at a port that refuses connections immediately, so the
/// blocking redis call fails fast with a connection error (no hang). The point
/// of these tests is the *control flow* (no nested-runtime panic), not the
/// specific Redis error.
fn unreachable_client() -> RedisOpenMontageClient {
    let client = redis::Client::open("redis://127.0.0.1:1").expect("redis url parses");
    RedisOpenMontageClient::new(client)
}

fn sample_envelope() -> WorkerEnvelope {
    WorkerEnvelope {
        task_id: "task-regression".to_string(),
        job_id: "job-regression".to_string(),
        project_id: "omx-job-regression".to_string(),
        attempt: 1,
        max_attempts: 3,
        kind: "run".to_string(),
        request_json: serde_json::json!({ "hello": "world" }),
        resume_from_stage: None,
        approval_decision_json: None,
        start_sequence: 1,
        enqueued_at: "2026-05-29T00:00:00Z".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enqueue_run_does_not_panic_within_async_runtime() {
    let client = unreachable_client();
    let res = client.enqueue_run(sample_envelope());
    assert!(
        res.is_err(),
        "enqueue_run must return a graceful Err against unreachable Redis (no nested-runtime panic), got: {res:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enqueue_resume_does_not_panic_within_async_runtime() {
    let client = unreachable_client();
    let res = client.enqueue_resume(sample_envelope());
    assert!(
        res.is_err(),
        "enqueue_resume must return a graceful Err against unreachable Redis (no nested-runtime panic), got: {res:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn set_cancel_flag_does_not_panic_within_async_runtime() {
    let client = unreachable_client();
    let res = client.set_cancel_flag("job-regression");
    assert!(
        res.is_err(),
        "set_cancel_flag must return a graceful Err against unreachable Redis (no nested-runtime panic), got: {res:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_preflight_does_not_panic_within_async_runtime() {
    let client = unreachable_client();
    let res = client.read_preflight();
    assert!(
        res.is_err(),
        "read_preflight must return a graceful Err against unreachable Redis (no nested-runtime panic), got: {res:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_pipelines_does_not_panic_within_async_runtime() {
    let client = unreachable_client();
    let res = client.read_pipelines();
    assert!(
        res.is_err(),
        "read_pipelines must return a graceful Err against unreachable Redis (no nested-runtime panic), got: {res:?}"
    );
}
