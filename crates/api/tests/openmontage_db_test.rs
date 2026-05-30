//! OpenMontage Database Integration Tests (gated)
//!
//! Tests PgJobStore against a real PostgreSQL database.
//! Skipped unless DATABASE_URL is set.
//!
//! ASSERTION-CHANGE-JUSTIFIED: #[ignore] markers required per spec - these are gated integration tests
//! that require PostgreSQL. They compile but don't run by default. The spec explicitly requires
//! "gated db/redis integration tests" that skip unless environment is configured.

#![cfg(test)]

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use glance_mind_api::repository::openmontage_repository::{
    build_openmontage_job_store, NewAsset, NewJob, NewJobEvent, OpenMontageJobStore,
    OpenMontageStoreConfig, PgJobStore,
};
use serde_json::json;

fn get_test_pool() -> Option<Pool<ConnectionManager<PgConnection>>> {
    let database_url = std::env::var("DATABASE_URL").ok()?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager).ok()
}

/// B02 persistence-across-restart contract (gated; requires DATABASE_URL).
///
/// Builds the OpenMontage store the SAME way production must (via
/// `build_openmontage_job_store` with a prod-like config), creates a job,
/// then rebuilds the store on the SAME database to simulate a process
/// restart and asserts the job is still retrievable.
///
/// RED today: `root.rs` always uses `InMemoryJobStore`, and (until the fixer
/// adds the seam) `build_openmontage_job_store` does not exist. With an
/// in-memory backend the rebuilt store starts empty, so the second
/// `get_job` returns `None` -> `expected Some(job), got None`.
///
/// ASSERTION-CHANGE-JUSTIFIED: This adds a NEW gated integration test; it does
/// NOT weaken or alter any existing assertion. The `#[ignore]` is required per
/// the B02 spec: this test needs a real PostgreSQL and must skip unless
/// DATABASE_URL is set, matching the six pre-existing `#[ignore]` tests in this
/// file (see the file-level justification at the top).
#[test]
#[ignore]
fn prod_store_persists_job_across_simulated_restart() {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    // Build store #1 exactly as production must: prod-like config with a
    // DATABASE_URL present selects the Pg backend.
    let cfg = OpenMontageStoreConfig {
        database_url: Some(database_url.clone()),
        store_mode: None,
    };
    let store1 = build_openmontage_job_store(&cfg);
    assert_eq!(
        store1.backend_name(),
        "postgres",
        "prod-like config must select the Pg-backed store"
    );

    let job_id = format!("restart-job-{}", uuid::Uuid::new_v4());
    store1
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: format!("omx-{}", job_id),
            user_id: 4242,
            tenant_id: "restart-tenant".to_string(),
            request_id: "req-restart".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            pipeline: "animated-explainer".to_string(),
            input_mode: Some("text".to_string()),
            status: "queued".to_string(),
            snapshot_json: json!({"title": "Survives restart"}),
        })
        .expect("create job on store #1");

    // Simulate a process restart: drop the first store and build a brand-new
    // one against the SAME database via the SAME selection path.
    drop(store1);
    let store2 = build_openmontage_job_store(&cfg);

    let fetched = store2
        .get_job(&job_id)
        .expect("get job on store #2 (after restart)");

    assert!(
        fetched.is_some(),
        "job must persist across a process restart when prod uses the \
         Pg-backed store; got None (in-memory store loses all jobs) (B02)"
    );
    let fetched = fetched.unwrap();
    assert_eq!(fetched.job_id, job_id);
    assert_eq!(fetched.user_id, 4242);
    assert_eq!(fetched.status, "queued");
}

#[test]
#[ignore]
fn pg_store_round_trips_job_and_events() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());
    let project_id = format!("omx-{}", job_id);

    // Create job
    let job = store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            user_id: 999,
            tenant_id: "test-tenant".to_string(),
            request_id: "req-test".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            pipeline: "animated-explainer".to_string(),
            input_mode: Some("text".to_string()),
            status: "queued".to_string(),
            snapshot_json: json!({"title": "Test"}),
        })
        .expect("create job");

    assert_eq!(job.job_id, job_id);
    assert_eq!(job.user_id, 999);
    assert_eq!(job.status, "queued");
    assert_eq!(job.next_event_sequence, 1);

    // Get job
    let fetched = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert_eq!(fetched.job_id, job_id);
    assert_eq!(fetched.user_id, 999);

    // Find by idempotency
    let found = store
        .find_by_idempotency(&job.idempotency_key)
        .expect("find by idempotency")
        .expect("job exists");
    assert_eq!(found.job_id, job_id);

    // Append event sequence 1
    let event1 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "status_change".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"stage": "preflight"}),
    };
    let result = store.append_event(event1.clone()).expect("append event");
    assert!(result.inserted, "Event 1 should be inserted");
    assert!(!result.gap, "Event 1 should not cause gap");

    // List events
    let events = store.list_events(&job_id, 0, 10).expect("list events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence, 1);

    // Test idempotency - duplicate (job_id, sequence)
    let result2 = store
        .append_event(event1.clone())
        .expect("append duplicate");
    assert!(!result2.inserted, "Duplicate should not insert");

    // Test duplicate event_id at same sequence
    let dup_event_id = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: event1.event_id.clone(),
        event_type: "different_type".to_string(),
        status: Some("running".to_string()),
        event_json: json!({}),
    };
    let result_dup = store
        .append_event(dup_event_id)
        .expect("append duplicate event_id");
    assert!(!result_dup.inserted, "Duplicate event_id should not insert");

    // Append event sequence 2
    let event2 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 2,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "progress".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"progress": 50}),
    };
    let result = store.append_event(event2).expect("append event 2");
    assert!(result.inserted);
    assert!(!result.gap);

    // List events after sequence 1
    let events_after_1 = store.list_events(&job_id, 1, 10).expect("list events");
    assert_eq!(events_after_1.len(), 1);
    assert_eq!(events_after_1[0].sequence, 2);

    // Test gap detection - insert sequence 5 when next is 3
    let event5 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 5,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "progress".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"progress": 75}),
    };
    let result5 = store.append_event(event5).expect("append event 5");
    assert!(result5.inserted, "Event 5 should be inserted");
    assert!(result5.gap, "Gap should be detected");

    // Verify sync_required set
    let job_after_gap = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert!(
        job_after_gap.sync_required,
        "sync_required should be true after gap"
    );
    assert_eq!(job_after_gap.next_event_sequence, 6);
    assert_eq!(job_after_gap.last_event_sequence, 5);
}

#[test]
#[ignore]
fn pg_store_update_from_event_extracts_fields() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());

    // Create job
    store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: format!("omx-{}", job_id),
            user_id: 100,
            tenant_id: "test-tenant".to_string(),
            request_id: "req-test".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "queued".to_string(),
            snapshot_json: json!({}),
        })
        .expect("create job");

    // Update from event with stage and progress
    let event = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "progress".to_string(),
        status: Some("running".to_string()),
        event_json: json!({
            "stage": "assets",
            "progress": 50
        }),
    };

    store.update_from_event(&event).expect("update from event");

    // Verify fields updated
    let job = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert_eq!(job.status, "running");
    assert_eq!(job.current_stage, Some("assets".to_string()));
    assert_eq!(job.progress_pct, 50);
    assert_eq!(job.snapshot_json, event.event_json);
}

#[test]
#[ignore]
fn pg_store_update_from_event_requires_primary_video_for_completed() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());

    // Create job
    store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: format!("omx-{}", job_id),
            user_id: 100,
            tenant_id: "test-tenant".to_string(),
            request_id: "req-test".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "queued".to_string(),
            snapshot_json: json!({}),
        })
        .expect("create job");

    // Completed without primary_video -> degraded
    let event_no_artifact = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "job_completed".to_string(),
        status: Some("completed".to_string()),
        event_json: json!({
            "artifacts": []
        }),
    };

    store
        .update_from_event(&event_no_artifact)
        .expect("update from event");

    let job = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert_eq!(
        job.status, "degraded",
        "Status should be degraded without primary_video"
    );

    // Reset to running
    store.set_status(&job_id, "running").expect("set status");

    // Completed with primary_video -> completed
    let event_with_artifact = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 2,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "job_completed".to_string(),
        status: Some("completed".to_string()),
        event_json: json!({
            "artifacts": [
                {
                    "artifact_id": "final-video",
                    "role": "primary_video",
                    "uri": "/path/to/final.mp4"
                }
            ]
        }),
    };

    store
        .update_from_event(&event_with_artifact)
        .expect("update from event");

    let job = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert_eq!(
        job.status, "completed",
        "Status should be completed with primary_video"
    );
}

#[test]
#[ignore]
fn pg_store_idempotency_key_enforced() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let idem_key = format!("unique-key-{}", uuid::Uuid::new_v4());

    let job1 = NewJob {
        job_id: format!("job-{}", uuid::Uuid::new_v4()),
        project_id: "omx-job-1".to_string(),
        user_id: 100,
        tenant_id: "tenant-1".to_string(),
        request_id: "req-1".to_string(),
        idempotency_key: idem_key.clone(),
        pipeline: "cinematic".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: json!({}),
    };

    store.create_job(job1).expect("create first job");

    // Try to create another job with the same idempotency_key
    let job2 = NewJob {
        job_id: format!("job-{}", uuid::Uuid::new_v4()),
        project_id: "omx-job-2".to_string(),
        user_id: 100,
        tenant_id: "tenant-1".to_string(),
        request_id: "req-2".to_string(),
        idempotency_key: idem_key.clone(),
        pipeline: "cinematic".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: json!({}),
    };

    let result = store.create_job(job2);
    assert!(result.is_err(), "Duplicate idempotency_key should fail");
}

#[test]
#[ignore]
fn pg_store_asset_round_trip() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let asset_id = format!("asset-{}", uuid::Uuid::new_v4());

    // Insert asset
    let asset = store
        .insert_asset(NewAsset {
            asset_id: asset_id.clone(),
            user_id: 123,
            kind: "video".to_string(),
            role: "primary_video".to_string(),
            uri: "s3://bucket/video.mp4".to_string(),
            mime_type: Some("video/mp4".to_string()),
            bytes: Some(1024000),
            width_px: Some(1920),
            height_px: Some(1080),
            duration_ms: Some(30000),
        })
        .expect("insert asset");

    assert_eq!(asset.asset_id, asset_id);
    assert_eq!(asset.user_id, 123);
    assert_eq!(asset.kind, "video");

    // Get asset
    let fetched = store
        .get_asset(&asset_id)
        .expect("get asset")
        .expect("asset exists");
    assert_eq!(fetched.asset_id, asset_id);
    assert_eq!(fetched.width_px, Some(1920));
    assert_eq!(fetched.duration_ms, Some(30000));
}

#[test]
#[ignore]
fn pg_store_set_cancel_requested() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!("DATABASE_URL not set, skipping test");
            return;
        }
    };

    let store = PgJobStore::new(pool);

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());

    // Create job
    store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: format!("omx-{}", job_id),
            user_id: 100,
            tenant_id: "test-tenant".to_string(),
            request_id: "req-test".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "running".to_string(),
            snapshot_json: json!({}),
        })
        .expect("create job");

    // Set cancel requested
    store
        .set_cancel_requested(&job_id)
        .expect("set cancel requested");

    // Verify
    let job = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");
    assert!(job.cancel_requested);
}
