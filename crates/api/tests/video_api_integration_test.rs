/**
 * Video API Integration Test
 *
 * Test flow:
 * 1. Register user and get JWT token
 * 2. Add credits (ensure sufficient balance)
 * 3. Upload image to get media_id
 * 4. Generate video using text
 * 5. Generate video using image
 * 6. Verify task records and credit deduction
 */
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use diesel::prelude::*;
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use glance_mind_api::repository::video_repository::VideoRepository;
use glance_mind_api::repository::wallet_repository::WalletRepository;
use hyper::body::to_bytes;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

// Note: Migrations are now managed by glance_mind_db project.
// Tests assume the database schema is already up to date.

fn setup_test_db() -> Arc<Database> {
    dotenv::dotenv().ok();
    Arc::new(Database::new())
}

async fn register_test_user(app: &axum::Router, email: &str, password: &str) -> (String, i32) {
    let username = format!(
        "vid{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..6]
    );

    let register_body = json!({
        "email": email,
        "password": password,
        "full_name": "Video Test User",
        "username": username,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&register_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let reg_body = to_bytes(response.into_body()).await.unwrap();
    if status != StatusCode::OK {
        let body_str = String::from_utf8(reg_body.to_vec()).unwrap();
        eprintln!("Registration failed: status={}, body={}", status, body_str);
        panic!("Registration failed");
    }
    assert_eq!(status, StatusCode::OK);

    // Parse registration response to get user_id
    let reg_json: Value = serde_json::from_slice(&reg_body).unwrap();
    let user_id = reg_json["user"]["id"].as_i64().unwrap() as i32;

    // Login to get token
    let login_body = json!({
        "identifier": email,
        "password": password,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_status = response.status();
    let body = to_bytes(response.into_body()).await.unwrap();
    if login_status != StatusCode::OK {
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        eprintln!("Login failed: status={}, body={}", login_status, body_str);
        panic!("Login failed");
    }
    assert_eq!(login_status, StatusCode::OK);

    // Parse token from login response
    let login_json: Value = serde_json::from_slice(&body).unwrap();
    let token = login_json["token"].as_str().unwrap().to_string();

    (token, user_id)
}

async fn add_balance(db: &Arc<Database>, uid: i32, points: i64) {
    let mut conn = db.pool.get().expect("Failed to get DB connection");

    use bigdecimal::BigDecimal;
    use glance_mind_api::schema::gm_user_wallets::dsl::*;

    diesel::update(gm_user_wallets.filter(user_id.eq(uid)))
        .set(balance_points.eq(balance_points + BigDecimal::from(points)))
        .execute(&mut conn)
        .expect("Failed to add balance");
}

// Create a minimal PNG image (1x1 pixel)
fn create_test_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15,
        0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01,
        0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

#[tokio::test]
async fn test_video_api_full_workflow() {
    println!("\n========== Video API Full Integration Test ==========\n");

    // 1. Setup test environment
    let db = setup_test_db();
    let app = app(db.clone());

    let test_email = format!("video_test_{}@example.com", uuid::Uuid::new_v4());
    let test_password = "TestPassword123!";

    println!("✓ Test environment setup complete");

    // 2. Register user and get token
    println!("→ Registering test user...");
    let (token, user_id) = register_test_user(&app, &test_email, test_password).await;
    println!(
        "✓ User registered, user_id={}, token={}",
        user_id,
        &token[..20]
    );

    // 3. Add credits (ensure sufficient balance)
    println!("→ Adding credits...");
    add_balance(&db, user_id, 5000).await;
    println!("✓ Added 5000 credits");

    // 4. Test image upload
    println!("\n--- Test 1: Upload Image ---");
    let png_data = create_test_png();

    // Create multipart form data
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut body_data = Vec::new();

    // Add file part
    body_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"image\"; filename=\"test.png\"\r\n",
    );
    body_data.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body_data.extend_from_slice(&png_data);
    body_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let upload_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/upload-image")
                .header("authorization", format!("Bearer {}", token))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body_data))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("Upload response status: {}", upload_response.status());

    let upload_body = hyper::body::to_bytes(upload_response.into_body())
        .await
        .unwrap();
    let upload_json: Value = serde_json::from_slice(&upload_body).unwrap();

    println!(
        "Upload response: {}",
        serde_json::to_string_pretty(&upload_json).unwrap()
    );

    // Verify response
    assert!(
        upload_json["code"].as_i64().unwrap() == 200
            || upload_json["success"].as_bool().unwrap_or(false),
        "Image upload failed: {:?}",
        upload_json
    );

    let media_id = if let Some(data) = upload_json["data"].as_object() {
        data["media_id"]
            .as_str()
            .expect("Missing media_id")
            .to_string()
    } else {
        upload_json["media_id"]
            .as_str()
            .expect("Missing media_id")
            .to_string()
    };

    println!("✓ Image upload successful, media_id: {}", media_id);

    // Verify credit deduction (should deduct 10 credits)
    let mut conn = db.pool.get().unwrap();
    let wallet = WalletRepository::get_by_user_id(&mut conn, user_id).unwrap();
    println!("  Current balance: {}", wallet.balance_points);

    // 5. Test video generation using text
    println!("\n--- Test 2: Generate Video Using Text ---");
    let create_video_body = json!({
        "prompt": "A cute cat playing with a ball of yarn",
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/generate")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&create_video_body).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("Create video response status: {}", create_response.status());

    let create_body = hyper::body::to_bytes(create_response.into_body())
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();

    println!(
        "Create video response: {}",
        serde_json::to_string_pretty(&create_json).unwrap()
    );

    assert!(
        create_json["code"].as_i64().unwrap() == 200
            || create_json["success"].as_bool().unwrap_or(false),
        "Create video task failed: {:?}",
        create_json
    );

    let task_id_1 = if let Some(data) = create_json["data"].as_object() {
        data["task_id"]
            .as_str()
            .expect("Missing task_id")
            .to_string()
    } else {
        create_json["task_id"]
            .as_str()
            .expect("Missing task_id")
            .to_string()
    };

    println!("✓ Video task created (text), task_id: {}", task_id_1);

    // Verify task record
    let task_1 = VideoRepository::get_by_task_id(&mut conn, &task_id_1).unwrap();
    assert_eq!(task_1.user_id, user_id);
    assert_eq!(task_1.status, "pending");
    assert!(task_1.prompt.is_some());
    assert!(task_1.media_id.is_none());
    println!("✓ Task record verified");

    // Verify credit pre-deduction (should freeze 200 credits)
    let wallet = WalletRepository::get_by_user_id(&mut conn, user_id).unwrap();
    println!(
        "  Current balance: {}, Frozen: {}",
        wallet.balance_points, wallet.frozen_points
    );

    // 6. Test video generation using image
    println!("\n--- Test 3: Generate Video Using Image ---");
    let create_video_body_2 = json!({
        "media_id": media_id,
        "prompt": "Make this image come alive with animation",
    });

    let create_response_2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/generate")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&create_video_body_2).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    println!(
        "Create video 2 response status: {}",
        create_response_2.status()
    );

    let create_body_2 = hyper::body::to_bytes(create_response_2.into_body())
        .await
        .unwrap();
    let create_json_2: Value = serde_json::from_slice(&create_body_2).unwrap();

    println!(
        "Create video 2 response: {}",
        serde_json::to_string_pretty(&create_json_2).unwrap()
    );

    assert!(
        create_json_2["code"].as_i64().unwrap() == 200
            || create_json_2["success"].as_bool().unwrap_or(false),
        "Create video task failed: {:?}",
        create_json_2
    );

    let task_id_2 = if let Some(data) = create_json_2["data"].as_object() {
        data["task_id"]
            .as_str()
            .expect("Missing task_id")
            .to_string()
    } else {
        create_json_2["task_id"]
            .as_str()
            .expect("Missing task_id")
            .to_string()
    };

    println!("✓ Video task created (image+text), task_id: {}", task_id_2);

    // Verify task record
    let task_2 = VideoRepository::get_by_task_id(&mut conn, &task_id_2).unwrap();
    assert_eq!(task_2.user_id, user_id);
    assert_eq!(task_2.status, "pending");
    assert!(task_2.prompt.is_some());
    assert!(task_2.media_id.is_some());
    assert_eq!(task_2.media_id.unwrap(), media_id);
    println!("✓ Task record verified");

    // 7. Test parameter validation (should fail)
    println!("\n--- Test 4: Parameter Validation ---");
    let invalid_body = json!({
        // Neither prompt nor media_id
    });

    let invalid_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/generate")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&invalid_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 error
    assert_eq!(
        invalid_response.status(),
        StatusCode::BAD_REQUEST,
        "Parameter validation should fail"
    );
    println!("✓ Parameter validation test passed (correctly rejected invalid request)");

    // 8. Verify final balance
    println!("\n--- Test 5: Balance Verification ---");
    let final_wallet = WalletRepository::get_by_user_id(&mut conn, user_id).unwrap();
    println!("  Final balance: {}", final_wallet.balance_points);
    println!("  Frozen credits: {}", final_wallet.frozen_points);

    // Verify deductions: upload 10 credits + two tasks pre-deduct 400 credits = total 410 credits
    // Balance should be: 5000 - 10 = 4990
    // Frozen should be: 400
    use bigdecimal::BigDecimal;
    let expected_balance = BigDecimal::from(4990);
    let expected_frozen = BigDecimal::from(400);

    assert_eq!(
        final_wallet.balance_points, expected_balance,
        "Balance mismatch, expected {}, actual {}",
        expected_balance, final_wallet.balance_points
    );
    assert_eq!(
        final_wallet.frozen_points, expected_frozen,
        "Frozen credits mismatch, expected {}, actual {}",
        expected_frozen, final_wallet.frozen_points
    );

    println!("✓ Balance verification passed");

    // 9. Final summary
    println!("\n========== Test Summary ==========");
    println!("✓ User registration and login");
    println!("✓ Image upload (deducted 10 credits)");
    println!("✓ Text-to-video generation (pre-deducted 200 credits)");
    println!("✓ Image+text-to-video generation (pre-deducted 200 credits)");
    println!("✓ Parameter validation");
    println!("✓ Credit deduction and freezing");
    println!("\nAll tests passed! ✨");
    println!("\nNote: Scheduler will process these tasks in background");
    println!("Task 1: {}", task_id_1);
    println!("Task 2: {}", task_id_2);
    println!("================================\n");
}

#[tokio::test]
async fn test_insufficient_balance() {
    println!("\n========== Test Insufficient Balance Scenario ==========\n");

    let db = setup_test_db();
    let app = app(db.clone());

    let test_email = format!("poor_user_{}@example.com", uuid::Uuid::new_v4());
    let test_password = "TestPassword123!";

    // Register user (balance is 0)
    let (token, user_id) = register_test_user(&app, &test_email, test_password).await;
    println!("✓ User registered, user_id={}", user_id);

    // Try to upload image (insufficient balance)
    let png_data = create_test_png();
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let mut body_data = Vec::new();

    body_data.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body_data.extend_from_slice(
        b"Content-Disposition: form-data; name=\"image\"; filename=\"test.png\"\r\n",
    );
    body_data.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body_data.extend_from_slice(&png_data);
    body_data.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/upload-image")
                .header("authorization", format!("Bearer {}", token))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(Body::from(body_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return error
    println!("Response status: {}", response.status());
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::PAYMENT_REQUIRED,
        "Insufficient balance should return 400 or 402 error"
    );

    println!("✓ Insufficient balance test passed");
    println!("================================\n");
}

#[tokio::test]
async fn test_video_generation_with_long_prompt() {
    println!("\n========== Test Long Prompt Validation ==========\n");

    let db = setup_test_db();
    let app = app(db.clone());

    let test_email = format!("long_prompt_{}@example.com", uuid::Uuid::new_v4());
    let test_password = "TestPassword123!";

    let (token, user_id) = register_test_user(&app, &test_email, test_password).await;
    add_balance(&db, user_id, 1000).await;

    // Create a prompt over 500 characters
    let long_prompt = "A".repeat(501);

    let create_body = json!({
        "prompt": long_prompt,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/video/generate")
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 error
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Long prompt should be rejected"
    );

    println!("✓ Long prompt validation test passed");
    println!("================================\n");
}
