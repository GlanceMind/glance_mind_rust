//! R014 M4-T3: Per-pipeline required asset enforcement (hybrid requires source_video)
//!
//! Tests that `create_job` rejects hybrid creates missing required asset roles (422),
//! does NOT enqueue them, and allows creates with the required roles present.

use glance_mind_api::{
    dto::openmontage_dto::CreateJobDto,
    repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
    service::{
        openmontage_client::MockOpenMontageClient, openmontage_service::OpenMontageService,
        openmontage_stream_hub::OpenMontageStreamHub,
    },
};
use proptest::prelude::*;
use std::sync::Arc;

/// Build a service with in-memory store and mock client
fn test_service() -> (
    OpenMontageService,
    Arc<InMemoryJobStore>,
    Arc<MockOpenMontageClient>,
) {
    let store = Arc::new(InMemoryJobStore::new());
    let client = Arc::new(MockOpenMontageClient::new());
    let hub = OpenMontageStreamHub::new();
    let service = OpenMontageService::new(store.clone(), client.clone(), hub.clone());
    (service, store, client)
}

/// RED: hybrid pipeline with NO source_video asset → 422 validation error + no enqueue
#[test]
fn hybrid_create_without_source_video_is_rejected() {
    let (service, _store, client) = test_service();

    // Create a hybrid job with NO asset_ids (no source_video)
    let dto = CreateJobDto {
        title: "Hybrid No Source".to_string(),
        prompt: "Should fail validation".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("hybrid".to_string()),
        input_mode: Some("source_clip".to_string()),
        asset_ids: Some(vec![]), // NO assets provided
        ..Default::default()
    };

    let result = service.create_job(1, "test-tenant", dto);

    // Assert: 422 validation error mentioning source_video
    assert!(result.is_err(), "Expected validation error, got success");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("source_video") || err_msg.contains("validation"),
        "Error should mention source_video or validation, got: {}",
        err_msg
    );

    // Assert: NO enqueue happened (enqueue count == 0)
    assert_eq!(
        client.enqueue_count(),
        0,
        "Job should NOT be enqueued when validation fails"
    );
}

/// GREEN: hybrid pipeline WITH source_video asset → 200/202 + enqueued
#[test]
fn hybrid_create_with_source_video_is_accepted() {
    let (service, store, client) = test_service();

    // Insert a source_video asset
    let asset = NewAsset {
        asset_id: "asset-123".to_string(),
        user_id: 1,
        kind: "source_video".to_string(),
        role: "source_video".to_string(),
        uri: "https://example.com/video.mp4".to_string(),
        mime_type: Some("video/mp4".to_string()),
        bytes: Some(1024),
        width_px: None,
        height_px: None,
        duration_ms: None,
    };
    store.insert_asset(asset).expect("insert asset failed");

    // Create a hybrid job WITH the source_video asset
    let dto = CreateJobDto {
        title: "Hybrid With Source".to_string(),
        prompt: "Should succeed".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("hybrid".to_string()),
        input_mode: Some("source_clip".to_string()),
        asset_ids: Some(vec!["asset-123".to_string()]),
        ..Default::default()
    };

    let result = service.create_job(1, "test-tenant", dto);

    // Assert: success (200/202)
    assert!(
        result.is_ok(),
        "Expected success with source_video, got error: {:?}",
        result
    );

    // Assert: job was enqueued (enqueue count == 1)
    assert_eq!(
        client.enqueue_count(),
        1,
        "Job should be enqueued when validation passes"
    );
}

/// GREEN: non-hybrid pipeline (cinematic) with NO assets → still OK (not blocked)
#[test]
fn non_hybrid_pipeline_with_no_assets_is_accepted() {
    let (service, _store, client) = test_service();

    // Create a cinematic job with NO assets (text_to_video mode)
    let dto = CreateJobDto {
        title: "Cinematic No Assets".to_string(),
        prompt: "Should succeed".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("cinematic".to_string()),
        input_mode: Some("text_to_video".to_string()),
        asset_ids: Some(vec![]), // NO assets
        ..Default::default()
    };

    let result = service.create_job(1, "test-tenant", dto);

    // Assert: success (cinematic doesn't require any assets)
    assert!(
        result.is_ok(),
        "Expected success for cinematic without assets, got error: {:?}",
        result
    );

    // Assert: job was enqueued
    assert_eq!(
        client.enqueue_count(),
        1,
        "Cinematic job should be enqueued"
    );
}

/// GREEN: animated-explainer with NO assets → success (no required roles)
#[test]
fn animated_explainer_with_no_assets_is_accepted() {
    let (service, _store, client) = test_service();

    let dto = CreateJobDto {
        title: "Explainer No Assets".to_string(),
        prompt: "Should succeed".to_string(),
        target_platform: "youtube".to_string(),
        pipeline: Some("animated-explainer".to_string()),
        input_mode: Some("text_to_video".to_string()),
        asset_ids: Some(vec![]), // NO assets
        ..Default::default()
    };

    let result = service.create_job(1, "test-tenant", dto);

    assert!(
        result.is_ok(),
        "Expected success for animated-explainer without assets, got error: {:?}",
        result
    );
    assert_eq!(client.enqueue_count(), 1);
}

proptest! {
    /// Property: hybrid create succeeds IFF at least one resolved asset has role=source_video
    #[test]
    fn prop_hybrid_create_ok_iff_source_video_present(
        title in ".{1,50}",
        has_source_video in proptest::bool::ANY,
    ) {
        let (service, store, client) = test_service();

        // Insert assets conditionally
        if has_source_video {
            let asset = NewAsset {
                asset_id: "source-asset".to_string(),
                user_id: 1,
                kind: "source_video".to_string(),
                role: "source_video".to_string(),
                uri: "https://example.com/source.mp4".to_string(),
                mime_type: Some("video/mp4".to_string()),
                bytes: Some(2048),
                width_px: None,
                height_px: None,
                duration_ms: None,
            };
            store.insert_asset(asset).expect("insert asset failed");
        }

        let dto = CreateJobDto {
            title: title.clone(),
            prompt: "Property test".to_string(),
            target_platform: "youtube".to_string(),
            pipeline: Some("hybrid".to_string()),
            input_mode: Some("source_clip".to_string()),
            asset_ids: if has_source_video {
                Some(vec!["source-asset".to_string()])
            } else {
                Some(vec![])
            },
            ..Default::default()
        };

        let result = service.create_job(1, "test-tenant", dto);

        if has_source_video {
            // Should succeed + enqueue
            prop_assert!(result.is_ok(), "Expected success with source_video");
            prop_assert_eq!(client.enqueue_count(), 1, "Should enqueue when valid");
        } else {
            // Should fail with validation error + NO enqueue
            prop_assert!(result.is_err(), "Expected validation error without source_video");
            let err_msg = result.unwrap_err();
            prop_assert!(
                err_msg.contains("source_video") || err_msg.contains("validation"),
                "Error should mention source_video: {}",
                err_msg
            );
            prop_assert_eq!(client.enqueue_count(), 0, "Should NOT enqueue when invalid");
        }
    }
}
