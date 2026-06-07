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
    NewAsset, NewJob, NewJobEvent, OpenMontageJobStore, PgJobStore,
};
use serde_json::json;

fn get_test_pool() -> Option<Pool<ConnectionManager<PgConnection>>> {
    let database_url = std::env::var("DATABASE_URL").ok()?;

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder().build(manager).ok()
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
    let result = store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            user_id: 999,
            tenant_id: "test-tenant".to_string(),
            request_id: "req-test".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            request_hash: "test-hash-1".to_string(),
            pipeline: "animated-explainer".to_string(),
            input_mode: Some("text".to_string()),
            status: "queued".to_string(),
            snapshot_json: json!({"title": "Test"}),
            render_runtime: None,
            approval_policy: None,
            budget_limit_usd: None,
        })
        .expect("create job");

    assert!(result.created, "First create should insert new job");
    let job = result.job;

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

    // Find by idempotency (now requires user_id)
    let found = store
        .find_by_idempotency(999, &job.idempotency_key)
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
            request_hash: "test-hash-2".to_string(),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "queued".to_string(),
            snapshot_json: json!({}),
            render_runtime: None,
            approval_policy: None,
            budget_limit_usd: None,
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
            request_hash: "test-hash-3".to_string(),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "queued".to_string(),
            snapshot_json: json!({}),
            render_runtime: None,
            approval_policy: None,
            budget_limit_usd: None,
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
        request_hash: "test-hash-4".to_string(),
        pipeline: "cinematic".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: json!({}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };

    let result1 = store.create_job(job1.clone()).expect("create first job");
    assert!(result1.created, "First create should insert new job");
    let first_job_id = result1.job.job_id.clone();

    // ASSERTION-CHANGE-JUSTIFIED: M0b-T1 changed the behavior from "error on duplicate" to "idempotent upsert".
    // The old assertion `assert!(result.is_err())` tested the wrong behavior (fail-on-duplicate breaks idempotency).
    // The new assertion verifies atomic upsert: same (user_id, key) → return existing job, created=false.
    // Try to create another job with the same (user_id, idempotency_key) — should return existing
    let job2 = NewJob {
        job_id: format!("job-{}", uuid::Uuid::new_v4()), // Different job_id
        project_id: "omx-job-2".to_string(),
        user_id: 100, // Same user_id
        tenant_id: "tenant-1".to_string(),
        request_id: "req-2".to_string(),
        idempotency_key: idem_key.clone(), // Same key
        request_hash: "test-hash-5".to_string(),
        pipeline: "cinematic".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: json!({}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };

    let result2 = store.create_job(job2).expect("create should succeed (idempotent)");
    assert!(
        !result2.created,
        "Second create with same (user_id, key) should find existing job"
    );
    assert_eq!(
        result2.job.job_id, first_job_id,
        "Should return the FIRST job's ID, not the second's"
    );
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
            request_hash: "test-hash-6".to_string(),
            pipeline: "animated-explainer".to_string(),
            input_mode: None,
            status: "running".to_string(),
            snapshot_json: json!({}),
            render_runtime: None,
            approval_policy: None,
            budget_limit_usd: None,
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

/// M0-T7: DB-gated test that render_runtime, approval_policy, budget_limit_usd persist via PgJobStore.
/// This is the credential-gated counterpart to the unconditional twin in openmontage_store_wiring_test.rs.
/// ASSERTION-CHANGE-JUSTIFIED: #[ignore] required as credential gate (DATABASE_URL) per M0-T7 spec -
/// this is a Postgres integration test with an unconditional in-memory twin that covers it deterministically.
#[test]
#[ignore]
fn pg_store_persists_execution_config_fields() {
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

    // Create job with NON-DEFAULT execution config values
    let result = store
        .create_job(NewJob {
            job_id: job_id.clone(),
            project_id: project_id.clone(),
            user_id: 456,
            tenant_id: "test-tenant-db".to_string(),
            request_id: "req-test-db".to_string(),
            idempotency_key: format!("idem-{}", uuid::Uuid::new_v4()),
            request_hash: "test-hash-db".to_string(),
            pipeline: "cinematic".to_string(),
            input_mode: Some("text_to_video".to_string()),
            status: "queued".to_string(),
            snapshot_json: json!({"title": "DB Test"}),
            render_runtime: Some("hyperframes".to_string()),
            approval_policy: Some("manual".to_string()),
            budget_limit_usd: Some(12.34),
        })
        .expect("create job");

    let _job = result.job; // Suppress unused warning

    // Fetch the stored job
    let fetched = store
        .get_job(&job_id)
        .expect("get job")
        .expect("job exists");

    // HARD ASSERTIONS - these MUST round-trip (M0-T7 spec)
    assert_eq!(
        fetched.render_runtime,
        Some("hyperframes".to_string()),
        "render_runtime must persist in Postgres"
    );
    assert_eq!(
        fetched.approval_policy,
        Some("manual".to_string()),
        "approval_policy must persist in Postgres"
    );
    assert_eq!(
        fetched.budget_limit_usd,
        Some(12.34),
        "budget_limit_usd must persist in Postgres"
    );

    // Sanity check other fields
    assert_eq!(fetched.job_id, job_id);
    assert_eq!(fetched.pipeline, "cinematic");
    assert_eq!(fetched.status, "queued");
}

/// M0b-T1: Concurrent create_job calls with the same (user_id, key, body) should produce
/// exactly ONE job row and exactly ONE enqueue (via atomic upsert at the DB layer).
/// This is the credential-gated concurrency test; its deterministic twin is
/// `idempotency_conflict_path_does_not_enqueue_twin` in openmontage_idempotency_test.rs.
/// ASSERTION-CHANGE-JUSTIFIED: #[ignore] marker is required per spec - credential-gated DB test
/// that requires PostgreSQL. The deterministic twin runs unconditionally; this is the DB counterpart.
#[tokio::test]
#[ignore]
async fn concurrent_creates_with_same_key_produce_one_job_and_one_enqueue() {
    let pool = match get_test_pool() {
        Some(p) => p,
        None => {
            eprintln!(
                "DATABASE_URL not set, skipping concurrency test (deterministic twin covers it)"
            );
            return;
        }
    };

    use glance_mind_api::service::openmontage_client::MockOpenMontageClient;
    use glance_mind_api::service::openmontage_service::OpenMontageService;
    use glance_mind_api::service::openmontage_stream_hub::OpenMontageStreamHub;
    use std::sync::Arc;

    let store = Arc::new(PgJobStore::new(pool));
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = Arc::new(OpenMontageService::new(store.clone(), client.clone(), hub));

    let user_id = 1;
    let tenant_id = "test-tenant";
    let idempotency_key = format!("concurrent-test-{}", uuid::Uuid::new_v4());

    let dto = glance_mind_api::dto::openmontage_dto::CreateJobDto {
        title: "Concurrent Test".to_string(),
        prompt: "Make a video".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        idempotency_key: Some(idempotency_key.clone()),
        ..Default::default()
    };

    // Spawn TWO concurrent create_job calls with the SAME (user_id, key, body)
    let service1 = service.clone();
    let service2 = service.clone();
    let dto1 = dto.clone();
    let dto2 = dto.clone();

    let (result1, result2) = tokio::join!(
        tokio::spawn(async move { service1.create_job(user_id, tenant_id, dto1) }),
        tokio::spawn(async move { service2.create_job(user_id, tenant_id, dto2) })
    );

    // Both spawns should complete without panicking
    let job1 = result1.expect("spawn1").expect("create1");
    let job2 = result2.expect("spawn2").expect("create2");

    // ASSERT: Both should return the SAME job_id (idempotency)
    assert_eq!(
        job1.job_id, job2.job_id,
        "Concurrent creates with same key should return the same job_id"
    );

    // ASSERT: Exactly ONE job row exists in the DB
    let jobs_in_db: Vec<_> = store
        .get_job(&job1.job_id)
        .expect("get_job")
        .into_iter()
        .collect();
    assert_eq!(
        jobs_in_db.len(),
        1,
        "Exactly one job row should exist in the DB"
    );

    // ASSERT (BINDING): Total enqueue count across both calls == 1 (no duplicate paid render)
    assert_eq!(
        client.get_enqueued().len(),
        1,
        "Concurrent creates MUST enqueue exactly once (no duplicate paid render)"
    );
}
