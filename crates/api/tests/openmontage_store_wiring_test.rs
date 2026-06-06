//! OpenMontage Store Wiring Tests (unconditional)
//!
//! Deterministic tests against InMemoryJobStore - NO DATABASE REQUIRED.
//! These are the "twin" tests that always run, with credential-gated DB tests as their counterpart.

#![cfg(test)]

use glance_mind_api::repository::openmontage_repository::{
    InMemoryJobStore, NewJob, OpenMontageJobStore,
};
use serde_json::json;

/// M0-T7: Test that render_runtime, approval_policy, budget_limit_usd round-trip from DTO -> create_job -> stored Job.
/// This is the TWIN (unconditional) test - DB-gated counterpart is in openmontage_db_test.rs.
#[test]
fn create_job_persists_execution_config_fields() {
    let store = InMemoryJobStore::new();

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());
    let project_id = format!("omx-{}", job_id);

    // Create job with NON-DEFAULT execution config values
    let new_job = NewJob {
        job_id: job_id.clone(),
        project_id: project_id.clone(),
        user_id: 123,
        tenant_id: "test-tenant".to_string(),
        request_id: "req-test".to_string(),
        idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
        request_hash: "test-hash-1".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text_to_video".to_string()),
        status: "queued".to_string(),
        snapshot_json: json!({"title": "Test"}),
        render_runtime: Some("hyperframes".to_string()),
        approval_policy: Some("manual".to_string()),
        budget_limit_usd: Some(7.5),
    };

    let _job = store.create_job(new_job).expect("create job");

    // Fetch the stored job
    let fetched = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");

    // HARD ASSERTIONS - these MUST round-trip (M0-T7 spec)
    // Currently FAILS because the insert path drops these three fields
    assert_eq!(
        fetched.render_runtime,
        Some("hyperframes".to_string()),
        "render_runtime must persist - currently drops to None"
    );
    assert_eq!(
        fetched.approval_policy,
        Some("manual".to_string()),
        "approval_policy must persist - currently drops to None"
    );
    assert_eq!(
        fetched.budget_limit_usd,
        Some(7.5),
        "budget_limit_usd must persist - currently drops to None"
    );

    // Sanity check other fields still work
    assert_eq!(fetched.job_id, job_id);
    assert_eq!(fetched.pipeline, "animated-explainer");
    assert_eq!(fetched.status, "queued");
}
