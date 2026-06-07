//! OpenMontage Per-Job Ownership Authorization Tests (M0-T6)
//!
//! Tests that only the user who owns a job (job.user_id == caller.id) can access it.
//! Each of the 5 read/mutate-by-id endpoints must return HTTP 403 for non-owners.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::{
    dto::openmontage_dto::ApprovalDto,
    repository::openmontage_repository::{InMemoryJobStore, NewJob, OpenMontageJobStore},
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use glance_mind_db::entity::user::User;
use std::sync::Arc;
use tower::ServiceExt;

/// Build a test router with a given user injected
fn test_router_with_user(user: User, service: OpenMontageService) -> axum::Router {
    use glance_mind_api::routes::openmontage;

    openmontage::user_routes()
        .layer(axum::Extension(user))
        .layer(axum::Extension(service))
}

fn mock_user(id: i32, email: &str) -> User {
    User {
        id,
        email: Some(email.to_string()),
        password_hash: "".to_string(),
        invitation_code: None,
        referred_by: None,
        company_name: None,
        api_key: None,
        status: "active".to_string(),
        full_name: format!("User {}", id),
        role: "user".to_string(),
        is_active: true,
        created_at: chrono::Utc::now(),
        updated_at: None,
        username: Some(format!("user{}", id)),
        permissions: 0,
    }
}

// ============================================================================
// Negative Tests: User B accessing User A's job → 403 + no data leak
// ============================================================================

#[tokio::test]
async fn get_job_returns_403_when_caller_is_not_owner() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");
    let user_b = mock_user(2, "bob@example.com");

    // Seed a job owned by user A
    let job_id = "job-owned-by-alice";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-123".to_string(),
        idempotency_key: "idem-123".to_string(),
        request_hash: "test-hash-123".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text".to_string()),
        status: "queued".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    // User B tries to get Alice's job
    let router = test_router_with_user(user_b, service);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}", job_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // MUST return 403 Forbidden
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "get_job must return 403 when caller is not the job owner"
    );

    // MUST NOT leak job data
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_ne!(
        resp["code"], 1000,
        "Response code should be error, not success"
    );
    assert!(
        resp["data"].is_null() || !resp.to_string().contains(job_id),
        "Response must not contain the job_id or job data (no leak)"
    );
}

#[tokio::test]
async fn get_events_returns_403_when_caller_is_not_owner() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");
    let user_b = mock_user(2, "bob@example.com");

    let job_id = "job-events-alice";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-456".to_string(),
        idempotency_key: "idem-456".to_string(),
        request_hash: "test-hash-456".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Events Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    let router = test_router_with_user(user_b, service);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}/events?after=0&limit=10", job_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "get_events must return 403 when caller is not the job owner"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        resp["data"].is_null() || !resp.to_string().contains(job_id),
        "Response must not leak event data"
    );
}

#[tokio::test]
async fn stream_job_returns_403_when_caller_is_not_owner() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");
    let user_b = mock_user(2, "bob@example.com");

    let job_id = "job-stream-alice";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-789".to_string(),
        idempotency_key: "idem-789".to_string(),
        request_hash: "test-hash-789".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "running".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Stream Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    let router = test_router_with_user(user_b, service);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}/stream", job_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "stream_job must return 403 when caller is not the job owner"
    );
}

#[tokio::test]
async fn submit_approval_returns_403_when_caller_is_not_owner() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");
    let user_b = mock_user(2, "bob@example.com");

    let job_id = "job-approval-alice";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-101".to_string(),
        idempotency_key: "idem-101".to_string(),
        request_hash: "test-hash-101".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "awaiting_approval".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Approval Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    let router = test_router_with_user(user_b, service);

    let approval = ApprovalDto {
        approval_id: "approval-999".to_string(),
        decision: "approve".to_string(),
        comment: Some("Bob trying to approve Alice's job".to_string()),
        revision_json: None,
    };

    let req = Request::builder()
        .method("POST")
        .uri(format!("/jobs/{}/approvals", job_id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&approval).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "submit_approval must return 403 when caller is not the job owner"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        resp["data"].is_null() || !resp.to_string().contains(job_id),
        "Response must not leak job data"
    );
}

#[tokio::test]
async fn cancel_job_returns_403_when_caller_is_not_owner() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");
    let user_b = mock_user(2, "bob@example.com");

    let job_id = "job-cancel-alice";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-202".to_string(),
        idempotency_key: "idem-202".to_string(),
        request_hash: "test-hash-202".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "running".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Cancel Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    let router = test_router_with_user(user_b, service);

    let req = Request::builder()
        .method("POST")
        .uri(format!("/jobs/{}/cancel", job_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "cancel_job must return 403 when caller is not the job owner"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        resp["data"].is_null() || !resp.to_string().contains(job_id),
        "Response must not leak job data"
    );
}

// ============================================================================
// Positive Test: Owner can access their own job
// ============================================================================

#[tokio::test]
async fn owner_can_access_their_own_job() {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user_a = mock_user(1, "alice@example.com");

    // Seed a job owned by user A
    let job_id = "job-owned-by-alice-positive";
    let new_job = NewJob {
        job_id: job_id.to_string(),
        project_id: format!("omx-{}", job_id),
        user_id: user_a.id,
        tenant_id: "default-tenant".to_string(),
        request_id: "req-positive".to_string(),
        idempotency_key: "idem-positive".to_string(),
        request_hash: "test-hash-positive".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: None,
        status: "queued".to_string(),
        snapshot_json: serde_json::json!({"title": "Alice's Own Job"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
    };
    OpenMontageJobStore::create_job(&*store, new_job).unwrap();

    // User A (owner) retrieves their own job
    let router = test_router_with_user(user_a, service);

    let req = Request::builder()
        .method("GET")
        .uri(format!("/jobs/{}", job_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // MUST return 200 OK
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Owner should be able to access their own job"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["code"], 1000, "Should return success code");
    assert_eq!(
        resp["data"]["job_id"], job_id,
        "Should return the correct job data"
    );
}
