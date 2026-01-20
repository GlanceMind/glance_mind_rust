use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// Note: Migrations are now managed by glance_mind_db project.
// Tests assume the database schema is already up to date.

#[tokio::test]
async fn test_upload_task_full_lifecycle() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize Database
    let db = Arc::new(Database::new());

    let app = app(db);

    // 1. Register and login
    let email = format!("upload_test_{}@example.com", uuid::Uuid::new_v4());
    let username = format!(
        "upload_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]
    );
    let password = "TestPassword123!";

    let register_payload = serde_json::json!({
        "email": email,
        "username": username,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(register_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Login to get token
    let login_payload = serde_json::json!({
        "identifier": email,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = token_json["token"].as_str().unwrap();
    let auth_header = format!("Bearer {}", token);

    // 2. Create a social account with device_id
    let device_id = format!("device_{}", uuid::Uuid::new_v4());
    let profile_name = "test_profile";

    let account_payload = serde_json::json!({
        "platform_id": 1,
        "username": "test_tiktok_user",
        "cookie": "test_cookie_session_data",
        "device_id": device_id,
        "profile_name": profile_name,
        "daily_max_replies": 10
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/accounts")
        .header("content-type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(account_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();

    if status != StatusCode::OK {
        panic!(
            "Create account failed. Status: {}, Body: {:?}",
            status,
            String::from_utf8_lossy(&body_bytes)
        );
    }

    let account_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let social_account_id = account_json["id"].as_i64().unwrap();

    // 3. Create an upload task
    let task_metadata = serde_json::json!({
        "video_url": "https://example.com/test_video.mp4",
        "description": "Test video #upload #automation",
        "location": "beijing"
    });

    let task_payload = serde_json::json!({
        "social_account_id": social_account_id,
        "metadata": task_metadata
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/upload-tasks")
        .header("content-type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(task_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();

    if status != StatusCode::OK {
        panic!(
            "Create task failed. Status: {}, Body: {:?}",
            status,
            String::from_utf8_lossy(&body_bytes)
        );
    }

    assert_eq!(status, StatusCode::OK);
    let task_json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify task response
    assert!(task_json["id"].is_number());
    assert_eq!(task_json["task_type"], "upload");
    assert_eq!(task_json["status"], "init");
    assert_eq!(task_json["social_account_id"], social_account_id);

    // Verify metadata was enriched with platform and profile_name
    let metadata = &task_json["metadata"];
    assert_eq!(metadata["video_url"], "https://example.com/test_video.mp4");
    assert_eq!(metadata["description"], "Test video #upload #automation");
    assert_eq!(metadata["location"], "beijing");
    assert!(metadata["platform"].is_string()); // Should be enriched
    assert_eq!(metadata["profile_name"], profile_name);

    // 4. Query tasks by device (public endpoint)
    let query_url = format!(
        "/api/v1/public/tasks/by-device?device_id={}&status=init",
        device_id
    );
    let req = Request::builder()
        .method("GET")
        .uri(&query_url)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let query_result: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify paginated response structure
    assert!(query_result["list"].is_array());
    assert_eq!(query_result["total"].as_i64().unwrap(), 1);
    assert_eq!(query_result["page"].as_i64().unwrap(), 1);

    // Verify task in response
    let tasks = query_result["list"].as_array().unwrap();
    assert_eq!(tasks.len(), 1);

    let device_task = &tasks[0];
    assert_eq!(device_task["task_type"], "upload");

    // Verify the response uses "meta" field (as specified)
    assert!(device_task["meta"].is_object());
    let meta = &device_task["meta"];
    assert_eq!(meta["video_url"], "https://example.com/test_video.mp4");
    assert_eq!(meta["description"], "Test video #upload #automation");
    assert_eq!(meta["location"], "beijing");
    assert!(meta["platform"].is_string());
    assert_eq!(meta["profile_name"], profile_name);

    // 5. Query without status filter - should still return the task
    let query_url = format!("/api/v1/public/tasks/by-device?device_id={}", device_id);
    let req = Request::builder()
        .method("GET")
        .uri(&query_url)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let query_result: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(query_result["total"].as_i64().unwrap(), 1);

    // 6. Query with different status - should return empty
    let query_url = format!(
        "/api/v1/public/tasks/by-device?device_id={}&status=done",
        device_id
    );
    let req = Request::builder()
        .method("GET")
        .uri(&query_url)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let query_result: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(query_result["total"].as_i64().unwrap(), 0);
    assert_eq!(query_result["list"].as_array().unwrap().len(), 0);

    // 7. List my tasks (authenticated endpoint)
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/upload-tasks/my?page=1&per_page=10")
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let my_tasks: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(my_tasks["total"].as_i64().unwrap(), 1);
    let my_task_list = my_tasks["list"].as_array().unwrap();
    assert_eq!(my_task_list.len(), 1);
    assert_eq!(my_task_list[0]["task_type"], "upload");

    println!("✅ All upload task tests passed!");
}

#[tokio::test]
async fn test_upload_task_error_cases() {
    dotenv::dotenv().ok();
    let db = Arc::new(Database::new());

    let app = app(db);

    // Register and login
    let email = format!("error_test_{}@example.com", uuid::Uuid::new_v4());
    let username = format!(
        "error_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]
    );
    let password = "TestPassword123!";

    let register_payload = serde_json::json!({
        "email": email,
        "username": username,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(register_payload.to_string()))
        .unwrap();

    let _ = app.clone().oneshot(req).await.unwrap();

    let login_payload = serde_json::json!({
        "identifier": email,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = token_json["token"].as_str().unwrap();
    let auth_header = format!("Bearer {}", token);

    // 1. Test creating task with non-existent social_account_id
    let task_payload = serde_json::json!({
        "social_account_id": 99999,
        "metadata": {
            "video_url": "https://example.com/video.mp4"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/upload-tasks")
        .header("content-type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(task_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    // Should return error (400 or 404)
    assert!(response.status().is_client_error());

    // 2. Test unauthorized access (no token)
    let task_payload = serde_json::json!({
        "social_account_id": 1,
        "metadata": {
            "video_url": "https://example.com/video.mp4"
        }
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/upload-tasks")
        .header("content-type", "application/json")
        .body(Body::from(task_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    // Should return 401 Unauthorized
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 3. Test querying with non-existent device_id
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/public/tasks/by-device?device_id=nonexistent_device")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let query_result: Value = serde_json::from_slice(&body_bytes).unwrap();
    // Should return empty list
    assert_eq!(query_result["total"].as_i64().unwrap(), 0);
    assert_eq!(query_result["list"].as_array().unwrap().len(), 0);

    println!("✅ All error case tests passed!");
}
