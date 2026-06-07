//! OpenMontage Redaction Tests (M0b-T5 rust half)
//!
//! Tests that:
//! 1. Resolved asset metadata containing secrets is rejected at create_job (422)
//! 2. Worker events with secrets are REDACTED before persistence + SSE publish
//! 3. Broadened SECRET_TOKENS catch common provider secrets

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::{
    dto::openmontage_dto::CreateJobDto,
    repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

/// Build a test router with seeded assets.
fn test_router_with_store(store: Arc<InMemoryJobStore>) -> axum::Router {
    use glance_mind_api::routes::openmontage;
    use glance_mind_db::entity::user::User;

    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());

    let user = User {
        id: 1,
        email: Some("test@example.com".to_string()),
        password_hash: "".to_string(),
        invitation_code: None,
        referred_by: None,
        company_name: None,
        api_key: None,
        status: "active".to_string(),
        full_name: "Test User".to_string(),
        role: "user".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: None,
        username: Some("testuser".to_string()),
        permissions: 0,
    };

    openmontage::user_routes()
        .layer(axum::Extension(user))
        .layer(axum::Extension(service))
}

/// Build internal callback router (no user auth).
fn test_callback_router_with_store(
    store: Arc<InMemoryJobStore>,
) -> (axum::Router, Arc<OpenMontageStreamHub>) {
    use glance_mind_api::routes::openmontage;

    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());

    (
        openmontage::internal_routes().layer(axum::Extension(service)),
        Arc::new(hub),
    )
}

// ============================================================================
// Test 1: Broadened SECRET_TOKENS catch common provider secrets
// ============================================================================

#[tokio::test]
async fn broadened_secret_tokens_reject_github_pat() {
    let store = Arc::new(InMemoryJobStore::new());
    let router = test_router_with_store(store);

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "My github token is ghp_1234567890abcdef".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    let status = response.status();
    if status != StatusCode::UNPROCESSABLE_ENTITY {
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8_lossy(&body);
        panic!("Expected 422, got {}. Body: {}", status, body_str);
    }
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should reject ghp_ prefix"
    );
}

#[tokio::test]
async fn broadened_secret_tokens_reject_aws_access_key() {
    let store = Arc::new(InMemoryJobStore::new());
    let router = test_router_with_store(store);

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "My AWS key is AKIAIOSFODNN7EXAMPLE".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should reject AKIA prefix"
    );
}

#[tokio::test]
async fn broadened_secret_tokens_reject_huggingface_token() {
    let store = Arc::new(InMemoryJobStore::new());
    let router = test_router_with_store(store);

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "My HF token is hf_abcdefghijklmnopqrstuvwxyz".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should reject hf_ prefix"
    );
}

#[tokio::test]
async fn broadened_secret_tokens_reject_slack_token() {
    let store = Arc::new(InMemoryJobStore::new());
    let router = test_router_with_store(store);

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        metadata: json!({
            "slack_webhook": "xoxb-1234567890-abcdefghijklmnopqrstuvwxyz"
        }),
        prompt: "Test".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should reject xoxb- prefix"
    );
}

// ============================================================================
// Test 2: Resolved asset metadata containing secrets is rejected (422)
// ============================================================================

#[tokio::test]
async fn create_job_rejects_resolved_asset_with_secret_in_uri() {
    let store = Arc::new(InMemoryJobStore::new());

    // Seed an asset with a secret in the URI
    let asset = NewAsset {
        asset_id: "asset-secret-uri".to_string(),
        user_id: 1,
        kind: "reference_image".to_string(),
        role: "primary_image".to_string(),
        uri: "s3://bucket/image.png?AWSAccessKeyId=AKIAIOSFODNN7EXAMPLE&Signature=xyz".to_string(),
        mime_type: Some("image/png".to_string()),
        bytes: Some(1024),
        width_px: None,
        height_px: None,
        duration_ms: None,
    };

    store.insert_asset(asset).unwrap();

    let router = test_router_with_store(store.clone());

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "Test prompt".to_string(),
        target_platform: "youtube".to_string(),
        asset_ids: Some(vec!["asset-secret-uri".to_string()]),
        input_mode: Some("image_to_video".to_string()),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // EXPECT: 422 rejection + NO enqueue
    assert_eq!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Should reject resolved asset with secret in uri"
    );

    // Verify NO job was created + enqueued
    let jobs = store.list_jobs_for_test();
    assert_eq!(jobs.len(), 0, "Should NOT have created a job");
}

#[tokio::test]
async fn create_job_accepts_clean_asset() {
    let store = Arc::new(InMemoryJobStore::new());

    // Seed a CLEAN asset (no secrets)
    let asset = NewAsset {
        asset_id: "asset-clean".to_string(),
        user_id: 1,
        kind: "reference_image".to_string(),
        role: "primary_image".to_string(),
        uri: "s3://bucket/image.png".to_string(),
        mime_type: Some("image/png".to_string()),
        bytes: Some(1024),
        width_px: None,
        height_px: None,
        duration_ms: None,
    };

    store.insert_asset(asset).unwrap();

    let router = test_router_with_store(store.clone());

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "Test prompt".to_string(),
        target_platform: "youtube".to_string(),
        asset_ids: Some(vec!["asset-clean".to_string()]),
        input_mode: Some("image_to_video".to_string()),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // EXPECT: 200 OK + job created + enqueued
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should accept clean asset"
    );

    let jobs = store.list_jobs_for_test();
    assert_eq!(jobs.len(), 1, "Should have created a job");
}

// ============================================================================
// Test 3: Worker events with secrets are REDACTED before persistence + SSE
// ============================================================================

#[tokio::test]
async fn ingest_event_redacts_secret_in_event_json() {
    let store = Arc::new(InMemoryJobStore::new());

    // Seed a job
    use glance_mind_api::repository::openmontage_repository::NewJob;
    let job = NewJob {
        job_id: "job-1".to_string(),
        project_id: "omx-job-1".to_string(),
        user_id: 1,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-1".to_string(),
        idempotency_key: "idem-1".to_string(),
        request_hash: "hash-1".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "running".to_string(),
        snapshot_json: json!({}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    let created = store.create_job(job).unwrap();
    assert!(created.created, "Job should be created");

    let (router, _hub) = test_callback_router_with_store(store.clone());

    // Worker event with a secret-shaped value in the payload
    let event = json!({
        "version": "v1",
        "event_id": "evt-1",
        "sequence": 1,
        "job": {
            "job_id": "job-1",
            "project_id": "omx-job-1",
            "request_id": "req-1",
            "correlation_id": "",
            "idempotency_key": "idem-1"
        },
        "event_type": "job_error",
        "status": "failed",
        "stage": "assets",
        "progress_pct": 0,
        "emitted_at": "2024-01-01T12:00:00.000000Z",
        "artifacts": [],
        "error": {
            "message": "Provider error: API key ghp_1234567890abcdef is invalid"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/callback")
        .header("content-type", "application/json")
        .header("x-internal-token", "test-secret")
        .body(Body::from(serde_json::to_string(&event).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the persisted event_json does NOT contain the literal secret
    let events = store.list_events("job-1", 0, 100).unwrap();
    assert_eq!(events.len(), 1);
    let persisted_event = &events[0];

    let event_json_str = persisted_event.event_json.to_string();
    assert!(
        !event_json_str.contains("ghp_1234567890abcdef"),
        "Persisted event_json MUST NOT contain literal secret; got: {}",
        event_json_str
    );
    assert!(
        event_json_str.contains("***REDACTED***"),
        "Persisted event_json MUST contain redaction marker; got: {}",
        event_json_str
    );
}

#[tokio::test]
async fn ingest_event_redacts_aws_key_in_nested_json() {
    let store = Arc::new(InMemoryJobStore::new());

    use glance_mind_api::repository::openmontage_repository::NewJob;
    let job = NewJob {
        job_id: "job-2".to_string(),
        project_id: "omx-job-2".to_string(),
        user_id: 1,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-2".to_string(),
        idempotency_key: "idem-2".to_string(),
        request_hash: "hash-2".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "running".to_string(),
        snapshot_json: json!({}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    store.create_job(job).unwrap();

    let (router, _hub) = test_callback_router_with_store(store.clone());

    let event = json!({
        "version": "v1",
        "event_id": "evt-2",
        "sequence": 1,
        "job": {
            "job_id": "job-2",
            "project_id": "omx-job-2",
            "request_id": "req-2",
            "correlation_id": "",
            "idempotency_key": "idem-2"
        },
        "event_type": "job_error",
        "status": "failed",
        "stage": "assets",
        "progress_pct": 0,
        "emitted_at": "2024-01-01T12:00:00.000000Z",
        "artifacts": [],
        "debug_info": {
            "provider_response": {
                "error": "InvalidAccessKeyId: The AWS Access Key Id AKIAIOSFODNN7EXAMPLE is not valid"
            }
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/callback")
        .header("content-type", "application/json")
        .header("x-internal-token", "test-secret")
        .body(Body::from(serde_json::to_string(&event).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = store.list_events("job-2", 0, 100).unwrap();
    assert_eq!(events.len(), 1);
    let event_json_str = events[0].event_json.to_string();

    assert!(
        !event_json_str.contains("AKIAIOSFODNN7EXAMPLE"),
        "AWS key must be redacted; got: {}",
        event_json_str
    );
    assert!(
        event_json_str.contains("***REDACTED***"),
        "Must contain redaction marker; got: {}",
        event_json_str
    );
}

#[tokio::test]
async fn ingest_event_clean_event_unchanged() {
    let store = Arc::new(InMemoryJobStore::new());

    use glance_mind_api::repository::openmontage_repository::NewJob;
    let job = NewJob {
        job_id: "job-3".to_string(),
        project_id: "omx-job-3".to_string(),
        user_id: 1,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-3".to_string(),
        idempotency_key: "idem-3".to_string(),
        request_hash: "hash-3".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "running".to_string(),
        snapshot_json: json!({}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    store.create_job(job).unwrap();

    let (router, _hub) = test_callback_router_with_store(store.clone());

    let event = json!({
        "version": "v1",
        "event_id": "evt-3",
        "sequence": 1,
        "job": {
            "job_id": "job-3",
            "project_id": "omx-job-3",
            "request_id": "req-3",
            "correlation_id": "",
            "idempotency_key": "idem-3"
        },
        "event_type": "job_progress",
        "status": "running",
        "stage": "assets",
        "progress_pct": 50,
        "emitted_at": "2024-01-01T12:00:00.000000Z",
        "artifacts": [],
        "message": "Processing assets"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/callback")
        .header("content-type", "application/json")
        .header("x-internal-token", "test-secret")
        .body(Body::from(serde_json::to_string(&event).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = store.list_events("job-3", 0, 100).unwrap();
    assert_eq!(events.len(), 1);
    let event_json_str = events[0].event_json.to_string();

    // Clean event should NOT contain redaction marker
    assert!(
        !event_json_str.contains("***REDACTED***"),
        "Clean event should not be redacted; got: {}",
        event_json_str
    );
    assert!(
        event_json_str.contains("Processing assets"),
        "Clean message should be preserved; got: {}",
        event_json_str
    );
}
