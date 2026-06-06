//! OpenMontage Idempotency Tests (M0-T5)
//!
//! Tests that create_job is idempotent:
//! - Same key + same body → return existing job_id, enqueue count == 1
//! - Same key + different body → HTTP 409 conflict, enqueue count == 1
//! - No key + identical body → derived key works, enqueue count == 1
//! - proptest property: identical key+body ⇒ same job_id AND single enqueue

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::{
    dto::openmontage_dto::CreateJobDto,
    repository::openmontage_repository::InMemoryJobStore,
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use std::sync::Arc;
use tower::ServiceExt;

/// Build a test router with a shared client + store so we can count enqueues across calls
fn test_router_with_shared_deps() -> (
    axum::Router,
    Arc<InMemoryJobStore>,
    Arc<MockOpenMontageClient>,
) {
    use glance_mind_api::routes::openmontage;
    use glance_mind_db::entity::user::User;

    let store = Arc::new(InMemoryJobStore::new());
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

    let router = openmontage::user_routes()
        .layer(axum::Extension(user))
        .layer(axum::Extension(service));

    (router, store, client)
}

#[tokio::test]
async fn create_job_with_same_key_and_body_returns_existing_job_enqueues_once() {
    let (router, _store, client) = test_router_with_shared_deps();

    let payload = CreateJobDto {
        title: "Idempotency Test".to_string(),
        prompt: "Make a cool video".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        idempotency_key: Some("idem-key-1".to_string()),
        ..Default::default()
    };

    // FIRST request
    let req1 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let resp1 = router.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let body1 = hyper::body::to_bytes(resp1.into_body()).await.unwrap();
    let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    let job_id_1 = json1["data"]["job_id"].as_str().unwrap();

    // SECOND request — identical payload + key
    let req2 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let resp2 = router.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let body2 = hyper::body::to_bytes(resp2.into_body()).await.unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let job_id_2 = json2["data"]["job_id"].as_str().unwrap();

    // ASSERT: Same job_id
    assert_eq!(
        job_id_1, job_id_2,
        "Identical key+body should return the same job_id"
    );

    // ASSERT: Only enqueued ONCE (the critical anti-duplicate-render assertion)
    assert_eq!(
        client.get_enqueued().len(),
        1,
        "Identical key+body should enqueue exactly once"
    );
}

#[tokio::test]
async fn create_job_with_same_key_different_body_returns_409_conflict() {
    let (router, _store, client) = test_router_with_shared_deps();

    let payload1 = CreateJobDto {
        title: "Original Title".to_string(),
        prompt: "Original prompt".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        idempotency_key: Some("idem-key-conflict".to_string()),
        ..Default::default()
    };

    // FIRST request
    let req1 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload1).unwrap()))
        .unwrap();

    let resp1 = router.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    // SECOND request — same key, DIFFERENT prompt
    let payload2 = CreateJobDto {
        title: "Original Title".to_string(),
        prompt: "CHANGED prompt".to_string(), // <— CHANGED
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        idempotency_key: Some("idem-key-conflict".to_string()),
        ..Default::default()
    };

    let req2 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload2).unwrap()))
        .unwrap();

    let resp2 = router.oneshot(req2).await.unwrap();

    // ASSERT: HTTP 409 Conflict
    assert_eq!(resp2.status(), StatusCode::CONFLICT);

    let body2 = hyper::body::to_bytes(resp2.into_body()).await.unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();

    // Check error indicates idempotency conflict
    let msg = json2["msg"].as_str().unwrap();
    let msg_lower = msg.to_lowercase();
    assert!(
        msg_lower.contains("idempotency") && msg_lower.contains("conflict"),
        "Expected idempotency conflict error, got: {}",
        msg
    );

    // ASSERT: Still only ONE enqueue (the first one), not two
    assert_eq!(
        client.get_enqueued().len(),
        1,
        "Conflict should NOT enqueue a second job"
    );
}

#[tokio::test]
async fn create_job_no_client_key_identical_body_derives_same_key() {
    let (router, _store, client) = test_router_with_shared_deps();

    let payload = CreateJobDto {
        title: "Derived Key Test".to_string(),
        prompt: "Test prompt".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        // NO idempotency_key — will be derived from body
        ..Default::default()
    };

    // FIRST request
    let req1 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let resp1 = router.clone().oneshot(req1).await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);

    let body1 = hyper::body::to_bytes(resp1.into_body()).await.unwrap();
    let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
    let job_id_1 = json1["data"]["job_id"].as_str().unwrap();

    // SECOND request — same payload, no key
    let req2 = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let resp2 = router.oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);

    let body2 = hyper::body::to_bytes(resp2.into_body()).await.unwrap();
    let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
    let job_id_2 = json2["data"]["job_id"].as_str().unwrap();

    // ASSERT: Same job_id (derived key worked)
    assert_eq!(
        job_id_1, job_id_2,
        "Identical body (no client key) should derive the same key and return the same job_id"
    );

    // ASSERT: Only enqueued ONCE
    assert_eq!(
        client.get_enqueued().len(),
        1,
        "Derived key path should also enqueue exactly once"
    );
}

// NOTE: property coverage (determinism / key-precedence / hash-exclusion) lives in the
// derive_idempotency_key proptest unit tests; these async cases are end-to-end integration coverage.
#[tokio::test]
async fn multiple_identical_requests_always_return_same_job_and_enqueue_once() {
    // Property test: for various payloads, identical key+body should return same job_id and enqueue once
    let test_cases = vec![
        (
            "Short Title",
            "Simple prompt",
            "youtube",
            "animated-explainer",
        ),
        (
            "Marketing Video",
            "Create engaging content",
            "tiktok",
            "animation",
        ),
        (
            "Product Demo",
            "Showcase our new feature",
            "instagram",
            "cinematic",
        ),
    ];

    for (title, prompt, platform, pipeline) in test_cases {
        let (router, _store, client) = test_router_with_shared_deps();

        let payload = CreateJobDto {
            title: title.to_string(),
            prompt: prompt.to_string(),
            target_platform: platform.to_string(),
            pipeline: Some(pipeline.to_string()),
            idempotency_key: Some(format!("test-key-{}", title)),
            ..Default::default()
        };

        // Call create_job TWICE with the SAME payload
        let req1 = Request::builder()
            .method("POST")
            .uri("/jobs")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();

        let resp1 = router.clone().oneshot(req1).await.unwrap();
        let body1 = hyper::body::to_bytes(resp1.into_body()).await.unwrap();
        let json1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
        let job_id_1 = json1["data"]["job_id"].as_str().unwrap().to_string();

        let req2 = Request::builder()
            .method("POST")
            .uri("/jobs")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&payload).unwrap()))
            .unwrap();

        let resp2 = router.oneshot(req2).await.unwrap();
        let body2 = hyper::body::to_bytes(resp2.into_body()).await.unwrap();
        let json2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
        let job_id_2 = json2["data"]["job_id"].as_str().unwrap().to_string();

        // Property 1: Same job_id
        assert_eq!(
            job_id_1, job_id_2,
            "Case '{}': identical key+body should return same job_id",
            title
        );

        // Property 2: Enqueue count == 1 (no duplicate paid render)
        assert_eq!(
            client.get_enqueued().len(),
            1,
            "Case '{}': identical key+body should enqueue exactly once",
            title
        );
    }
}
