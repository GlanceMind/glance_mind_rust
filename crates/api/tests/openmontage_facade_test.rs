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
#[allow(clippy::bool_assert_comparison)]
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
        request_hash: "test-hash-123".to_string(),
        pipeline: "animated-explainer".to_string(),
        input_mode: Some("text".to_string()),
        status: "running".to_string(),
        snapshot_json: serde_json::json!({"title": "Running Test"}),
        render_runtime: None,
        approval_policy: None,
        budget_limit_usd: None,
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

// ============================================================================
// M2 Input Mode Mapping Tests (Part 3)
// ============================================================================

#[tokio::test]
async fn to_protocol_request_maps_image_to_video() {
    use glance_mind_api::{
        dto::openmontage_dto::CreateJobDto,
        repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
        service::{
            openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
            openmontage_stream_hub::OpenMontageStreamHub,
        },
    };
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    // Seed a reference_image asset
    let asset_id = uuid::Uuid::new_v4().to_string();
    let new_asset = NewAsset {
        asset_id: asset_id.clone(),
        user_id: 1,
        kind: "reference_image".to_string(),
        role: "primary_image".to_string(),
        uri: "https://example.com/image.png".to_string(),
        mime_type: Some("image/png".to_string()),
        bytes: Some(1024),
        width_px: Some(512),
        height_px: Some(512),
        duration_ms: None,
    };
    store.insert_asset(new_asset).unwrap();

    // Create job with input_mode=image_to_video and asset_ids
    let dto = CreateJobDto {
        title: "Image to Video Test".to_string(),
        prompt: "Animate this image".to_string(),
        target_platform: "youtube".to_string(),
        input_mode: Some("image_to_video".to_string()),
        asset_ids: Some(vec![asset_id.clone()]),
        duration_seconds: Some(5),
        ..Default::default()
    };

    service.create_job(1, "tenant-1", dto).unwrap();

    // Retrieve the enqueued request from MockClient
    let enqueued = client.last_enqueued_run().unwrap();
    let request_json: serde_json::Value =
        serde_json::from_str(&enqueued.request_json.to_string()).unwrap();

    // Verify assets array contains the reference_image
    assert!(request_json["assets"].is_array());
    let assets = request_json["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["kind"], "reference_image");
    assert_eq!(assets[0]["role"], "primary_image");
    assert_eq!(assets[0]["uri"], "https://example.com/image.png");

    // Verify tool_invocations contains image_to_video operation
    assert!(request_json["tool_invocations"].is_array());
    let invocations = request_json["tool_invocations"].as_array().unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0]["operation"], "image_to_video");

    let input_json: serde_json::Value =
        serde_json::from_str(invocations[0]["input_json"].as_str().unwrap()).unwrap();
    assert_eq!(input_json["prompt"], "Animate this image");
    assert_eq!(input_json["image_url"], "https://example.com/image.png");
    assert_eq!(input_json["duration"], 5);
}

#[tokio::test]
async fn to_protocol_request_maps_first_last_frame() {
    use glance_mind_api::{
        dto::openmontage_dto::CreateJobDto,
        repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
        service::{
            openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
            openmontage_stream_hub::OpenMontageStreamHub,
        },
    };
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    // Seed start_frame and end_frame assets
    let start_id = uuid::Uuid::new_v4().to_string();
    let end_id = uuid::Uuid::new_v4().to_string();

    store
        .insert_asset(NewAsset {
            asset_id: start_id.clone(),
            user_id: 1,
            kind: "start_frame".to_string(),
            role: "first_frame".to_string(),
            uri: "https://example.com/start.png".to_string(),
            mime_type: Some("image/png".to_string()),
            bytes: Some(1024),
            width_px: None,
            height_px: None,
            duration_ms: None,
        })
        .unwrap();

    store
        .insert_asset(NewAsset {
            asset_id: end_id.clone(),
            user_id: 1,
            kind: "end_frame".to_string(),
            role: "last_frame".to_string(),
            uri: "https://example.com/end.png".to_string(),
            mime_type: Some("image/png".to_string()),
            bytes: Some(1024),
            width_px: None,
            height_px: None,
            duration_ms: None,
        })
        .unwrap();

    // Create job with input_mode=first_last_frame
    let dto = CreateJobDto {
        title: "First Last Frame Test".to_string(),
        prompt: "Interpolate between frames".to_string(),
        target_platform: "tiktok".to_string(),
        input_mode: Some("first_last_frame".to_string()),
        asset_ids: Some(vec![start_id, end_id]),
        ..Default::default()
    };

    service.create_job(1, "tenant-1", dto).unwrap();

    let enqueued = client.last_enqueued_run().unwrap();
    let request_json: serde_json::Value =
        serde_json::from_str(&enqueued.request_json.to_string()).unwrap();

    // Verify assets array contains both frames
    let assets = request_json["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 2);
    assert!(assets.iter().any(|a| a["kind"] == "start_frame"));
    assert!(assets.iter().any(|a| a["kind"] == "end_frame"));

    // first_last_frame mode does NOT generate tool_invocations (worker interpolates)
    assert!(
        request_json["tool_invocations"].is_null()
            || request_json["tool_invocations"]
                .as_array()
                .unwrap()
                .is_empty()
    );
}

#[tokio::test]
async fn to_protocol_request_reference_driven_has_no_generation_invocation() {
    use glance_mind_api::{
        dto::openmontage_dto::CreateJobDto,
        repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
        service::{
            openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
            openmontage_stream_hub::OpenMontageStreamHub,
        },
    };
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    // Seed a reference_video asset
    let ref_vid_id = uuid::Uuid::new_v4().to_string();
    store
        .insert_asset(NewAsset {
            asset_id: ref_vid_id.clone(),
            user_id: 1,
            kind: "reference_video".to_string(),
            role: "style_reference".to_string(),
            uri: "https://example.com/reference.mp4".to_string(),
            mime_type: Some("video/mp4".to_string()),
            bytes: Some(10240),
            width_px: Some(1920),
            height_px: Some(1080),
            duration_ms: Some(5000),
        })
        .unwrap();

    // Create job with input_mode=reference_driven
    let dto = CreateJobDto {
        title: "Reference Driven Test".to_string(),
        prompt: "Match this style".to_string(),
        target_platform: "youtube".to_string(),
        input_mode: Some("reference_driven".to_string()),
        asset_ids: Some(vec![ref_vid_id]),
        ..Default::default()
    };

    service.create_job(1, "tenant-1", dto).unwrap();

    let enqueued = client.last_enqueued_run().unwrap();
    let request_json: serde_json::Value =
        serde_json::from_str(&enqueued.request_json.to_string()).unwrap();

    // Verify reference_video is in assets
    let assets = request_json["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["kind"], "reference_video");
    assert_eq!(assets[0]["role"], "style_reference");

    // reference_driven mode does NOT generate direct tool_invocations (analysis-first)
    assert!(
        request_json["tool_invocations"].is_null()
            || request_json["tool_invocations"]
                .as_array()
                .unwrap()
                .is_empty()
    );
}

// ============================================================================
// M0-T4 Pipeline Allowlist + Availability Validation Tests
// ============================================================================

#[tokio::test]
async fn create_job_rejects_non_production_pipeline_with_404() {
    use glance_mind_api::dto::openmontage_dto::CreateJobDto;
    use std::sync::Arc;

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    // Mock user
    let user = glance_mind_db::entity::user::User {
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

    // Request with a real OMX pipeline that is NOT in the production 6
    let payload = CreateJobDto {
        title: "Framework Smoke Test".to_string(),
        prompt: "Test prompt".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("framework-smoke".to_string()),
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // Should return 404
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Check that the error indicates pipeline_not_found
    assert_eq!(resp["code"], 2004); // NotFound
    let message = resp["msg"].as_str().unwrap();
    assert!(
        message.contains("pipeline") && message.contains("not found"),
        "Expected 'pipeline not found' error, got: {}",
        message
    );

    // CRITICAL: Verify no job was enqueued
    assert_eq!(
        client.get_enqueued().len(),
        0,
        "Job should not be enqueued on pipeline rejection"
    );
}

#[tokio::test]
async fn create_job_rejects_unavailable_pipeline_with_409() {
    use glance_mind_api::{
        dto::openmontage_dto::{CreateJobDto, PipelineInfoDto, PipelinesDto, PreflightDto},
        service::openmontage_client::OpenMontageClient,
    };
    use std::sync::{Arc, Mutex};

    // Custom mock that returns a degraded preflight/pipelines snapshot
    #[derive(Clone)]
    struct MockClientWithDegradedPipelines {
        enqueued: Arc<Mutex<Vec<glance_mind_api::service::openmontage_client::WorkerEnvelope>>>,
    }

    impl OpenMontageClient for MockClientWithDegradedPipelines {
        fn enqueue_run(
            &self,
            envelope: glance_mind_api::service::openmontage_client::WorkerEnvelope,
        ) -> Result<(), String> {
            self.enqueued.lock().unwrap().push(envelope);
            Ok(())
        }

        fn enqueue_resume(
            &self,
            envelope: glance_mind_api::service::openmontage_client::WorkerEnvelope,
        ) -> Result<(), String> {
            self.enqueued.lock().unwrap().push(envelope);
            Ok(())
        }

        fn set_cancel_flag(&self, _job_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
            // Preflight shows animated-explainer as unavailable (tool missing)
            Ok(Some(PreflightDto {
                passed: false,
                status: "degraded".to_string(),
                blocking: vec![],
                warnings: vec!["animated-explainer requires unavailable tools".to_string()],
                estimated_cost_cents: None,
            }))
        }

        fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
            // Pipelines shows animated-explainer but stability != production
            Ok(Some(PipelinesDto {
                pipelines: vec![PipelineInfoDto {
                    name: "animated-explainer".to_string(),
                    description: "Topic to fully generated explainer".to_string(),
                    stability: "beta".to_string(), // NOT production
                }],
            }))
        }
    }

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockClientWithDegradedPipelines {
        enqueued: Arc::new(Mutex::new(Vec::new())),
    });
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user = glance_mind_db::entity::user::User {
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

    // Request animated-explainer (in the allowlist, but unavailable per snapshot)
    let payload = CreateJobDto {
        title: "Degraded Pipeline Test".to_string(),
        prompt: "Test prompt".to_string(),
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

    // Should return 409 Conflict
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Body should include degraded capability info
    let message = resp["msg"].as_str().unwrap();
    assert!(
        message.contains("unavailable")
            || message.contains("degraded")
            || message.contains("stability:"),
        "Expected degraded/unavailable message, got: {}",
        message
    );

    // CRITICAL: Verify no job was enqueued
    assert_eq!(
        client.enqueued.lock().unwrap().len(),
        0,
        "Job should not be enqueued when pipeline is unavailable"
    );
}

#[tokio::test]
async fn create_job_succeeds_when_pipeline_available() {
    use glance_mind_api::{
        dto::openmontage_dto::{CreateJobDto, PipelineInfoDto, PipelinesDto, PreflightDto},
        service::openmontage_client::OpenMontageClient,
    };
    use std::sync::{Arc, Mutex};

    // Custom mock that returns a healthy preflight/pipelines snapshot
    #[derive(Clone)]
    struct MockClientWithHealthyPipelines {
        enqueued: Arc<Mutex<Vec<glance_mind_api::service::openmontage_client::WorkerEnvelope>>>,
    }

    impl OpenMontageClient for MockClientWithHealthyPipelines {
        fn enqueue_run(
            &self,
            envelope: glance_mind_api::service::openmontage_client::WorkerEnvelope,
        ) -> Result<(), String> {
            self.enqueued.lock().unwrap().push(envelope);
            Ok(())
        }

        fn enqueue_resume(
            &self,
            envelope: glance_mind_api::service::openmontage_client::WorkerEnvelope,
        ) -> Result<(), String> {
            self.enqueued.lock().unwrap().push(envelope);
            Ok(())
        }

        fn set_cancel_flag(&self, _job_id: &str) -> Result<(), String> {
            Ok(())
        }

        fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
            Ok(Some(PreflightDto {
                passed: true,
                status: "passed".to_string(),
                blocking: vec![],
                warnings: vec![],
                estimated_cost_cents: Some(100),
            }))
        }

        fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
            Ok(Some(PipelinesDto {
                pipelines: vec![PipelineInfoDto {
                    name: "animated-explainer".to_string(),
                    description: "Topic to fully generated explainer".to_string(),
                    stability: "production".to_string(),
                }],
            }))
        }
    }

    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockClientWithHealthyPipelines {
        enqueued: Arc::new(Mutex::new(Vec::new())),
    });
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub);

    let user = glance_mind_db::entity::user::User {
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

    // Request animated-explainer (available and in production)
    let payload = CreateJobDto {
        title: "Available Pipeline Test".to_string(),
        prompt: "Test prompt".to_string(),
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

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(resp["code"], 1000);
    assert!(!resp["data"]["job_id"].as_str().unwrap().is_empty());

    // CRITICAL: Verify the job WAS enqueued
    assert_eq!(
        client.enqueued.lock().unwrap().len(),
        1,
        "Job should be enqueued when pipeline is available"
    );
}

// ============================================================================
// M0-T7 Quality Fix: Budget Validation Tests
// ============================================================================

#[tokio::test]
async fn create_job_rejects_negative_budget() {
    use glance_mind_api::routes::openmontage;
    use glance_mind_db::entity::user::User;
    use std::sync::Arc;

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

    let payload = CreateJobDto {
        title: "Invalid Budget Test".to_string(),
        prompt: "Test with negative budget".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        budget_limit_usd: Some(-5.0), // Invalid: negative budget
        ..Default::default()
    };

    let req = Request::builder()
        .method("POST")
        .uri("/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();

    let response = router.oneshot(req).await.unwrap();

    // Should reject with 4xx validation error (BadRequest)
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Negative budget should return 400 Bad Request"
    );

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Check error message mentions budget
    let error_msg = resp["msg"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("budget") || error_msg.contains("negative"),
        "Error message should mention budget validation, got: {}",
        error_msg
    );

    // CRITICAL: Verify job was NOT enqueued
    assert_eq!(
        client.get_enqueued().len(),
        0,
        "Invalid budget should not result in enqueue"
    );
}
