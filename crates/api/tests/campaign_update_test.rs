use axum::body::Body;
use axum::http::{Request, StatusCode};
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use glance_mind_api::config::parameter;
use glance_mind_api::repository::user_repository::{UserRepository, UserRepositoryTrait};
use glance_mind_api::repository::wallet_repository::WalletRepository;
use hyper::body::to_bytes;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_update_campaign_region() {
    // Initialize parameters
    parameter::init();

    // Create database connection
    let db = Arc::new(Database::new());
    let app = app(db.clone());

    // Setup repositories
    let user_repo = UserRepository::new(db.pool.clone());
    let wallet_repo = WalletRepository::new(db.pool.clone());

    // Create unique test user for this test run
    use uuid::Uuid;
    let unique_suffix = Uuid::new_v4().to_string()[..8].to_string();
    let test_email = format!("campaign_region_test_{}@example.com", unique_suffix);
    let test_password = "password";
    let password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzS6rKZU6W"; // bcrypt hash of "password"

    println!("Creating test user: {}", test_email);
    let user = user_repo
        .create(
            Some(test_email.to_string()),
            Some(format!("test_{}", unique_suffix)),
            password_hash.to_string(),
            None,
            None,
        )
        .await
        .expect("Failed to create test user");

    // Create wallet
    wallet_repo
        .create(user.id)
        .await
        .expect("Failed to create wallet");

    println!("Created test user with ID: {}", user.id);

    // Login to get JWT token
    let login_payload = json!({
        "identifier": test_email,
        "password": test_password
    });

    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
        .unwrap();

    let login_res = app.clone().oneshot(login_req).await.unwrap();
    let login_status = login_res.status();
    let login_body = to_bytes(login_res.into_body()).await.unwrap();

    if login_status != StatusCode::OK {
        let error_msg = String::from_utf8_lossy(&login_body);
        panic!("Login failed with status {}: {}", login_status, error_msg);
    }

    let login_json: serde_json::Value = serde_json::from_slice(&login_body).unwrap();
    let token = login_json["access_token"].as_str().unwrap();

    // 1. Create a campaign with region_id = 7 (Global)
    let create_payload = json!({
        "name": "Test Campaign for Region Update",
        "platform_id": 2,
        "region_id": 7,
        "ai_model_id": 1,
        "social_group_id": 1,
        "product_prompt": "Test product description",
        "target_audience": "Test audience",
        "keyword": "test_keyword",
        "enable_ai_refactor": true,
        "max_scan_count": 100,
        "budget_cap": "50.00",
        "schedule_type": "INTERVAL",
        "schedule_config": {
            "interval_seconds": 300
        },
        "auto_like": true,
        "auto_follow": true,
        "auto_dm": true
    });

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/v1/campaigns")
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&create_payload).unwrap()))
        .unwrap();

    let create_res = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_res.status(), StatusCode::OK);

    let create_body = hyper::body::to_bytes(create_res.into_body()).await.unwrap();
    let created_campaign: serde_json::Value = serde_json::from_slice(&create_body).unwrap();
    let campaign_id = created_campaign["id"].as_i64().unwrap();

    println!("✅ Created campaign with ID: {}, region_id: 7", campaign_id);

    // 2. Verify initial region_id is 7
    assert_eq!(created_campaign["region_id"].as_i64().unwrap(), 7);
    println!("✅ Verified initial region_id is 7 (Global)");

    // 3. Update campaign to region_id = 11 (China)
    let update_payload = json!({
        "name": "Test Campaign for Region Update",
        "platform_id": 2,
        "region_id": 11,  // Update to China
        "ai_model_id": 1,
        "social_group_id": 1,
        "product_prompt": "Test product description",
        "target_audience": "Test audience",
        "keyword": "test_keyword",
        "enable_ai_refactor": true,
        "max_scan_count": 100,
        "budget_cap": "50.00",
        "schedule_type": "INTERVAL",
        "schedule_config": {
            "interval_seconds": 300
        },
        "auto_like": true,
        "auto_follow": true,
        "auto_dm": true
    });

    let update_req = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/campaigns/{}", campaign_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
        .unwrap();

    let update_res = app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(update_res.status(), StatusCode::OK);

    let update_body = hyper::body::to_bytes(update_res.into_body()).await.unwrap();
    let updated_campaign: serde_json::Value = serde_json::from_slice(&update_body).unwrap();

    println!(
        "✅ Updated campaign, new region_id: {}",
        updated_campaign["region_id"]
    );

    // 4. Verify region_id has been updated to 11
    assert_eq!(
        updated_campaign["region_id"].as_i64().unwrap(),
        11,
        "region_id should be updated to 11 (China)"
    );

    println!("✅ Successfully verified region_id updated from 7 (Global) to 11 (China)");

    // 5. Fetch campaign again to double-check persistence
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/campaigns/{}", campaign_id))
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let get_res = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);

    let get_body = hyper::body::to_bytes(get_res.into_body()).await.unwrap();
    let fetched_campaign: serde_json::Value = serde_json::from_slice(&get_body).unwrap();

    assert_eq!(
        fetched_campaign["region_id"].as_i64().unwrap(),
        11,
        "region_id should persist as 11 after fetching"
    );

    println!("✅ Verified region_id persists as 11 (China) after re-fetching");
}
