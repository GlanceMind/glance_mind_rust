//! OpenMontage Facade - Database Integration Tests
//!
//! Gated behind #[ignore] - requires a Postgres instance with migrations applied.
//! Run with: cargo test -p glance_mind_api --test openmontage_db_test -- --ignored

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use glance_mind_api::repository::openmontage_repository::*;
use serde_json::json;

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

fn get_test_pool() -> Pool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5434/openmontage_test".to_string());

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool")
}

#[test]
#[ignore]
fn pg_store_round_trips_job_and_events() {
    let pool = get_test_pool();
    let store = PgJobStore::new(pool);

    let job_id = format!("test-job-{}", uuid::Uuid::new_v4());
    let project_id = format!("omx-{}", job_id);

    // Create job
    store.create_job(NewJob {
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
    }).expect("create job");

    // Get job
    let job = store.get_job(&job_id).expect("get job").expect("job exists");
    assert_eq!(job.job_id, job_id);
    assert_eq!(job.user_id, 999);
    assert_eq!(job.status, "queued");

    // Append event
    let event1 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "status_change".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"stage": "preflight"}),
    };
    let result = store.append_event(event1.clone()).expect("append event");
    assert!(result.inserted);
    assert!(!result.gap);

    // List events
    let events = store.list_events(&job_id, 0, 10).expect("list events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence, 1);

    // Test idempotency
    let result2 = store.append_event(event1.clone()).expect("append duplicate");
    assert!(!result2.inserted, "Duplicate should not insert");

    // Test gap detection
    let event3 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 3,
        event_id: format!("evt-{}", uuid::Uuid::new_v4()),
        event_type: "progress".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"progress": 75}),
    };
    let result3 = store.append_event(event3).expect("append event 3");
    assert!(result3.inserted);
    assert!(result3.gap, "Gap should be detected");

    // Verify sync_required set
    let job_after_gap = store.get_job(&job_id).expect("get job").expect("job exists");
    assert!(job_after_gap.sync_required);
    assert_eq!(job_after_gap.next_event_sequence, 4);
}

#[test]
#[ignore]
fn pg_store_idempotency_key_enforced() {
    let pool = get_test_pool();
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
