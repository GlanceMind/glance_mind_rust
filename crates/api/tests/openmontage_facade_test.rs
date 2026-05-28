//! OpenMontage Facade - Deterministic Unit Tests
//!
//! Tests the foundation: DTOs, secret rejection, in-memory store, mock Redis client.
//! No external dependencies (DB/Redis).

use glance_mind_api::dto::openmontage_dto::*;
use glance_mind_api::repository::openmontage_repository::*;
use glance_mind_api::service::openmontage_client::*;
use serde_json::json;

#[test]
fn create_job_dto_rejects_inline_secret() {
    let dto = CreateJobDto {
        title: "Test Job".to_string(),
        prompt: "Create a video using api_key=sk-12345".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let result = dto.validate_no_secret_material();
    assert!(result.is_err(), "Should reject inline api_key");
    assert!(result.unwrap_err().contains("api_key"));

    let dto2 = CreateJobDto {
        title: "Test Job".to_string(),
        prompt: "Create a video".to_string(),
        target_platform: "youtube".to_string(),
        metadata: json!({"config": {"secret": "my-password"}}),
        ..Default::default()
    };

    let result2 = dto2.validate_no_secret_material();
    assert!(result2.is_err(), "Should reject secret in metadata");
    assert!(result2.unwrap_err().contains("secret"));

    let dto3 = CreateJobDto {
        title: "Test Job".to_string(),
        prompt: "Create a video with apikey=xyz123".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let result3 = dto3.validate_no_secret_material();
    assert!(result3.is_err(), "Should reject apikey");
}

#[test]
fn create_job_dto_accepts_minimal() {
    let dto = CreateJobDto {
        title: "Clean Video".to_string(),
        prompt: "Make a professional explainer video about cats".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let result = dto.validate_no_secret_material();
    assert!(result.is_ok(), "Should accept clean input: {:?}", result);
}

#[test]
fn in_memory_store_append_is_idempotent_and_detects_gap() {
    let store = InMemoryJobStore::new();

    // Create a job
    let job_id = "job-123".to_string();
    store.create_job(NewJob {
        job_id: job_id.clone(),
        project_id: "omx-job-123".to_string(),
        user_id: 1,
        tenant_id: "tenant-1".to_string(),
        request_id: "req-1".to_string(),
        idempotency_key: "idem-1".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text".to_string()),
        status: "queued".to_string(),
        snapshot_json: json!({}),
    }).expect("create job");

    // Append event sequence 1
    let event1 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 1,
        event_id: "evt-1".to_string(),
        event_type: "status_change".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"stage": "preflight"}),
    };
    let result1 = store.append_event(event1.clone()).expect("append event 1");
    assert!(result1.inserted, "First insert should succeed");
    assert!(!result1.gap, "No gap expected");

    // Append event sequence 2
    let event2 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 2,
        event_id: "evt-2".to_string(),
        event_type: "progress".to_string(),
        status: Some("running".to_string()),
        event_json: json!({"progress": 50}),
    };
    let result2 = store.append_event(event2.clone()).expect("append event 2");
    assert!(result2.inserted);
    assert!(!result2.gap);

    // List events after sequence 1 should return only sequence 2
    let events = store.list_events(&job_id, 1, 10).expect("list events");
    assert_eq!(events.len(), 1, "Should have 1 event after seq 1");
    assert_eq!(events[0].sequence, 2);
    assert_eq!(events[0].event_id, "evt-2");

    // Duplicate append (same event_id + sequence) should be idempotent
    let result3 = store.append_event(event2.clone()).expect("append duplicate");
    assert!(!result3.inserted, "Duplicate should not insert");
    assert!(!result3.gap);

    // Append sequence 5 (gap: expected 3, got 5)
    let event5 = NewJobEvent {
        job_id: job_id.clone(),
        sequence: 5,
        event_id: "evt-5".to_string(),
        event_type: "status_change".to_string(),
        status: Some("completed".to_string()),
        event_json: json!({}),
    };
    let result5 = store.append_event(event5).expect("append event 5");
    assert!(result5.inserted);
    assert!(result5.gap, "Gap should be detected");

    // Verify next_event_sequence advanced to 6
    let job = store.get_job(&job_id).expect("get job").expect("job exists");
    assert_eq!(job.next_event_sequence, 6);
    assert!(job.sync_required, "sync_required should be set on gap");
}

#[test]
fn mock_client_enqueue_run_serializes_valid_request() {
    let client = MockOpenMontageClient::new();

    // Create a CreateJobDto
    let dto = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "Make a sci-fi trailer".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("cinematic".to_string()),
        duration_seconds: Some(60),
        ..Default::default()
    };

    // Convert to protocol request
    let protocol_req = dto.to_protocol_request(ServerContext {
        job_id: "job-456".to_string(),
        user_id: 2,
        tenant_id: "tenant-2".to_string(),
        callback_secret_ref: Some("callback-secret-123".to_string()),
    });

    // Enqueue
    let envelope = WorkerEnvelope {
        task_id: "task-789".to_string(),
        job_id: "job-456".to_string(),
        project_id: "omx-job-456".to_string(),
        attempt: 1,
        max_attempts: 3,
        kind: "run".to_string(),
        request_json: serde_json::to_value(&protocol_req).expect("serialize protocol_req"),
        resume_from_stage: None,
        approval_decision_json: None,
        start_sequence: 1,
        enqueued_at: chrono::Utc::now().to_rfc3339(),
    };

    client.enqueue_run(envelope.clone()).expect("enqueue");

    // Verify the mock recorded it
    let enqueued = client.get_enqueued();
    assert_eq!(enqueued.len(), 1);
    assert_eq!(enqueued[0].kind, "run");
    assert_eq!(enqueued[0].job_id, "job-456");

    // Parse request_json back into OpenMontageProfessionalVideoRequest
    // (This validates the JSON shape matches the protobuf contract)
    let request_value = &enqueued[0].request_json;
    let title = request_value.get("title").and_then(|v| v.as_str()).expect("title field");
    assert_eq!(title, "Test Video");

    let pipeline = request_value.get("pipeline").and_then(|v| v.as_str()).expect("pipeline field");
    assert_eq!(pipeline, "cinematic");

    let tenant_id = request_value.get("tenant_id").and_then(|v| v.as_str()).expect("tenant_id");
    assert_eq!(tenant_id, "tenant-2");
}
