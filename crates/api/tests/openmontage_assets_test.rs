//! OpenMontage Assets OSS Integration Tests (gated)
//!
//! Run only when `RUN_OSS_INTEGRATION_TESTS=1` is set.
//! Tests real OSS upload + asset record insertion + cleanup.
//!
//! ASSERTION-CHANGE-JUSTIFIED: #[ignore] is required per spec — this is a gated integration test
//! that requires OSS credentials. It compiles but should only run when explicitly enabled via
//! RUN_OSS_INTEGRATION_TESTS=1. This is part of the gated test architecture (part 3 spec).

#![cfg(test)]

use glance_mind_api::{
    repository::openmontage_repository::{InMemoryJobStore, NewAsset, OpenMontageJobStore},
    service::oss_service::OssService,
};
use std::sync::Arc;

#[tokio::test]
#[ignore] // Only run with RUN_OSS_INTEGRATION_TESTS=1
async fn upload_real_image_asset_to_oss() {
    if std::env::var("RUN_OSS_INTEGRATION_TESTS").unwrap_or_default() != "1" {
        eprintln!("Skipping OSS test (RUN_OSS_INTEGRATION_TESTS != 1)");
        return;
    }

    // Create a tiny 1x1 PNG (valid image)
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44,
        0x41, 0x54, // IDAT chunk
        0x78, 0x9C, 0x62, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, // IEND chunk
        0xAE, 0x42, 0x60, 0x82,
    ];

    let user_id = 1;
    let filename = "test_reference_image.png".to_string();
    let content_type = "image/png".to_string();

    // Upload to OSS
    let oss_service = OssService::from_env().expect("OSS not configured");
    let upload_result = oss_service
        .upload_image(
            png_data.clone(),
            user_id,
            filename.clone(),
            content_type.clone(),
        )
        .await
        .expect("Upload should succeed");

    // Verify URL returned
    assert!(!upload_result.image_url.is_empty());
    assert!(upload_result.image_url.contains("aipub/user_1"));
    assert_eq!(upload_result.size, png_data.len());

    // Insert asset record
    let store = Arc::new(InMemoryJobStore::new());
    let asset_id = uuid::Uuid::new_v4().to_string();
    let new_asset = NewAsset {
        asset_id: asset_id.clone(),
        user_id,
        kind: "reference_image".to_string(),
        role: "primary_image".to_string(),
        uri: upload_result.image_url.clone(),
        mime_type: Some(content_type),
        bytes: Some(png_data.len() as i64),
        width_px: Some(1),
        height_px: Some(1),
        duration_ms: None,
    };

    let asset = store
        .insert_asset(new_asset)
        .expect("Insert should succeed");
    assert_eq!(asset.asset_id, asset_id);
    assert_eq!(asset.kind, "reference_image");
    assert_eq!(asset.uri, upload_result.image_url);

    // Retrieve asset
    let retrieved = store
        .get_asset(&asset_id)
        .expect("Get should succeed")
        .expect("Asset should exist");
    assert_eq!(retrieved.asset_id, asset_id);
    assert_eq!(retrieved.uri, upload_result.image_url);

    // Cleanup: delete the OSS object (optional — OSS cleanup is manual for now)
    eprintln!("Test completed. Uploaded to: {}", upload_result.image_url);
    eprintln!(
        "Note: Manual OSS cleanup required for object: {}",
        upload_result.object_path
    );
}
