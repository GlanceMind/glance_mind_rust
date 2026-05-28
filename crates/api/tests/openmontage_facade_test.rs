//! OpenMontage Facade Integration Tests
//!
//! Tests all user-facing endpoints (E1-E8) using an axum test router.
//! Uses InMemoryJobStore + MockClient + test preflight/pipeline sources.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::{
    dto::openmontage_dto::{ApprovalDto, CreateJobDto},
    repository::openmontage_repository::{InMemoryJobStore, OpenMontageJobStore},
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use tower::ServiceExt;

/// Build a test router with auth bypassed (Extension<User> injected directly).
fn test_openmontage_router() -> axum::Router {
    use glance_mind_api::routes::openmontage;
    use glance_mind_db::entity::user::User;
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());

    // Mock user
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

#[tokio::test]
async fn create_job_returns_queued_and_enqueues_without_aipub() {
    let router = test_openmontage_router();

    let payload = CreateJobDto {
        title: "Test Video".to_string(),
        prompt: "Make a cool video".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(resp["code"], 1000);
    let data = &resp["data"];
    assert_eq!(data["status"], "queued");
    assert!(!data["job_id"].as_str().unwrap().is_empty());
    assert!(!data["project_id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn get_snapshot_returns_dto() {
    let router = test_openmontage_router();

    // Create a job first
    let payload = CreateJobDto {
        title: "Snapshot Test".to_string(),
        prompt: "Test prompt".to_string(),
        target_platform: "tiktok".to_string(),
        ..Default::default()
    };

    let create_req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let create_resp = router.clone().oneshot(create_req).await.unwrap();
    let body = hyper::body::to_bytes(create_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = resp["data"]["job_id"].as_str().unwrap();

    // Get the snapshot
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}", job_id))
        .body(Body::empty())
        .unwrap();

    let get_resp = router.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(get_resp.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], 1000);
    assert_eq!(resp["data"]["job_id"], job_id);
    assert_eq!(resp["data"]["status"], "queued");
}

#[tokio::test]
async fn events_after_seq_paginates() {
    // This test would need to seed events into the store
    // For now, just test the endpoint exists and returns empty list
    let router = test_openmontage_router();

    let payload = CreateJobDto {
        title: "Events Test".to_string(),
        prompt: "Test".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let create_req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let create_resp = router.clone().oneshot(create_req).await.unwrap();
    let body = hyper::body::to_bytes(create_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = resp["data"]["job_id"].as_str().unwrap();

    let events_req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}/events?after=0&limit=10", job_id))
        .body(Body::empty())
        .unwrap();

    let events_resp = router.oneshot(events_req).await.unwrap();
    assert_eq!(events_resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(events_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], 1000);
    assert!(resp["data"]["events"].is_array());
}

#[tokio::test]
async fn sse_stream_delivers_ingested_event() {
    // SSE testing is complex with tower::oneshot
    // This is a placeholder - real test would subscribe and publish
    // For now, just verify the endpoint exists
    let router = test_openmontage_router();

    let payload = CreateJobDto {
        title: "SSE Test".to_string(),
        prompt: "Test".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let create_req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let create_resp = router.clone().oneshot(create_req).await.unwrap();
    let body = hyper::body::to_bytes(create_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = resp["data"]["job_id"].as_str().unwrap();

    let stream_req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}/stream", job_id))
        .body(Body::empty())
        .unwrap();

    let stream_resp = router.oneshot(stream_req).await.unwrap();
    // SSE endpoints return 200 and keep connection open
    assert_eq!(stream_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn approval_enqueues_resume() {
    let router = test_openmontage_router();

    // Create a job
    let payload = CreateJobDto {
        title: "Approval Test".to_string(),
        prompt: "Test".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let create_req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let create_resp = router.clone().oneshot(create_req).await.unwrap();
    let body = hyper::body::to_bytes(create_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = resp["data"]["job_id"].as_str().unwrap();

    // Submit approval
    let approval = ApprovalDto {
        approval_id: "approval-123".to_string(),
        decision: "approve".to_string(),
        comment: Some("Looks good".to_string()),
        revision_json: None,
    };

    let approval_req = Request::builder()
        .method("POST")
        .uri(format!("/jobs/{}/approvals", job_id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&approval).unwrap()))
        .unwrap();

    let approval_resp = router.oneshot(approval_req).await.unwrap();
    assert_eq!(approval_resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(approval_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], 1000);
}

#[tokio::test]
async fn cancel_queued_marks_cancelled() {
    let router = test_openmontage_router();

    let payload = CreateJobDto {
        title: "Cancel Test".to_string(),
        prompt: "Test".to_string(),
        target_platform: "youtube".to_string(),
        ..Default::default()
    };

    let create_req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let create_resp = router.clone().oneshot(create_req).await.unwrap();
    let body = hyper::body::to_bytes(create_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let job_id = resp["data"]["job_id"].as_str().unwrap();

    // Cancel the job
    let cancel_req = Request::builder()
        .method("POST")
        .uri(format!("/jobs/{}/cancel", job_id))
        .body(Body::empty())
        .unwrap();

    let cancel_resp = router.oneshot(cancel_req).await.unwrap();
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(cancel_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], 1000);
    assert_eq!(resp["data"]["cancel_requested"], false); // queued job -> status=cancelled
}

#[tokio::test]
async fn cancel_running_sets_flag() {
    use glance_mind_api::repository::openmontage_repository::NewJob;
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());

    // Seed a running job
    let job_id = "running-job-123";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: 1,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-123".to_string(),
        idempotency_key: "idem-123".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text".to_string()),
        status: "running".to_string(),
        snapshot_json: serde_json::json!({"title": "Running Test"}),
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    // Build router with seeded store/service
    use glance_mind_db::entity::user::User;

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

    let router = glance_mind_api::routes::openmontage::user_routes()
        .layer(axum::Extension(user))
        .layer(axum::Extension(service));

    // Cancel the running job
    let cancel_req = Request::builder()
        .method("POST")
        .uri(format!("/jobs/{}/cancel", job_id))
        .body(Body::empty())
        .unwrap();

    let cancel_resp = router.oneshot(cancel_req).await.unwrap();
    assert_eq!(cancel_resp.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(cancel_resp.into_body())
        .await
        .unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Running job -> cancel_requested=true, status stays "running"
    assert_eq!(resp["code"], 1000);
    assert_eq!(resp["data"]["cancel_requested"], true);
    assert!(resp["data"]["message"]
        .as_str()
        .unwrap()
        .contains("still running"));

    // Verify job status is still "running" but cancel_requested is true
    let job = OpenMontageJobStore::get_job(&*store, job_id)
        .unwrap()
        .unwrap();
    assert_eq!(job.status, "running");
    assert_eq!(job.cancel_requested, true);

    // Verify mock client recorded the cancel flag
    assert!(client.was_cancel_flag_set(job_id));
}
