use axum::body::Body;
use axum::http::{Request, StatusCode};
use diesel::prelude::*;
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// Note: Migrations are now managed by glance_mind_db project.
// Tests assume the database schema is already up to date.

#[tokio::test]
async fn test_campaign_export_functionality() {
    dotenv::dotenv().ok();
    let db = Arc::new(Database::new());
    let pool = &db.pool;
    let mut conn = pool.get().expect("Failed to get DB connection");

    let app = app(db.clone());

    // 1. Setup - Register and Login
    let email = format!("export_test_{}@test.com", uuid::Uuid::new_v4());
    let password = "Password123!";

    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "email": email,
                "username": format!("export_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]),
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(reg_req).await.unwrap();

    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/auth")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "identifier": email,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(login_res.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = token_json["token"].as_str().unwrap();
    let auth_header = format!("Bearer {}", token);

    // 2. Get config data for campaign creation
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/platforms")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let platforms: Value = serde_json::from_slice(&body_bytes).unwrap();
    let platform_id = platforms[0]["id"].as_i64().unwrap() as i32;

    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/config/platforms/{}/regions", platform_id))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let regions: Value = serde_json::from_slice(&body_bytes).unwrap();
    let region_id = regions[0]["id"].as_i64().unwrap() as i32;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/ai-models")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let models: Value = serde_json::from_slice(&body_bytes).unwrap();
    let ai_model_id = models[0]["id"].as_i64().unwrap() as i32;

    // 3. Create Campaign
    let campaign_payload = serde_json::json!({
        "name": "Export Test Campaign",
        "platform_id": platform_id,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": "Test Product"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/campaigns")
        .header("content-type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(campaign_payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let campaign: Value = serde_json::from_slice(&body_bytes).unwrap();
    let campaign_id = campaign["id"].as_i64().unwrap() as i32;

    // 4. Insert test data directly to database
    use glance_mind_api::schema::{gm_agent_comments, gm_agent_videos, gm_crawler_tasks};

    // Create a crawler task first
    let task_id: i32 = diesel::insert_into(gm_crawler_tasks::table)
        .values((
            gm_crawler_tasks::campaign_id.eq(campaign_id),
            gm_crawler_tasks::status.eq("COMPLETED"),
            gm_crawler_tasks::max_count.eq(100),
            gm_crawler_tasks::process_count.eq(2),
            gm_crawler_tasks::search_offset.eq(0),
            gm_crawler_tasks::search_limit.eq(100),
        ))
        .returning(gm_crawler_tasks::id)
        .get_result(&mut conn)
        .expect("Failed to insert crawler task");

    // Insert test videos
    let video_ids: Vec<i32> = diesel::insert_into(gm_agent_videos::table)
        .values(vec![
            (
                gm_agent_videos::task_id.eq(task_id),
                gm_agent_videos::campaign_id.eq(Some(campaign_id)),
                gm_agent_videos::video_id.eq(Some("test_video_001")),
                gm_agent_videos::author.eq(Some("test_author_1")),
                gm_agent_videos::description.eq(Some("Test video description 1")),
            ),
            (
                gm_agent_videos::task_id.eq(task_id),
                gm_agent_videos::campaign_id.eq(Some(campaign_id)),
                gm_agent_videos::video_id.eq(Some("test_video_002")),
                gm_agent_videos::author.eq(Some("test_author_2")),
                gm_agent_videos::description.eq(Some("Test video description 2")),
            ),
        ])
        .returning(gm_agent_videos::id)
        .get_results(&mut conn)
        .expect("Failed to insert videos");

    // Insert test comments
    diesel::insert_into(gm_agent_comments::table)
        .values(vec![
            (
                gm_agent_comments::video_db_id.eq(video_ids[0]),
                gm_agent_comments::campaign_id.eq(Some(campaign_id)),
                gm_agent_comments::comment_id.eq("comment_001"),
                gm_agent_comments::user_nickname.eq(Some("user1")),
                gm_agent_comments::user_unique_id.eq(Some("unique_001")),
                gm_agent_comments::content.eq(Some("This is a test comment")),
                gm_agent_comments::reason.eq(Some("Potential customer")),
                gm_agent_comments::suggested_reply.eq(Some("Thank you for your interest!")),
                gm_agent_comments::status.eq(0_i16), // Pending
            ),
            (
                gm_agent_comments::video_db_id.eq(video_ids[1]),
                gm_agent_comments::campaign_id.eq(Some(campaign_id)),
                gm_agent_comments::comment_id.eq("comment_002"),
                gm_agent_comments::user_nickname.eq(Some("user2")),
                gm_agent_comments::user_unique_id.eq(Some("unique_002")),
                gm_agent_comments::content.eq(Some("Another test comment")),
                gm_agent_comments::reason.eq(Some("Question about product")),
                gm_agent_comments::suggested_reply.eq(Some("Let me help you with that")),
                gm_agent_comments::status.eq(1_i16), // Approved
            ),
            (
                gm_agent_comments::video_db_id.eq(video_ids[0]),
                gm_agent_comments::campaign_id.eq(Some(campaign_id)),
                gm_agent_comments::comment_id.eq("comment_003"),
                gm_agent_comments::user_nickname.eq(Some("user3")),
                gm_agent_comments::user_unique_id.eq(Some("unique_003")),
                gm_agent_comments::content.eq(Some("Third test comment")),
                gm_agent_comments::reason.eq(Some("Spam")),
                gm_agent_comments::suggested_reply.eq(None),
                gm_agent_comments::status.eq(2_i16), // Rejected
            ),
        ])
        .execute(&mut conn)
        .expect("Failed to insert comments");

    // 5. Test Export - should succeed
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/campaigns/{}/export", campaign_id))
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();

    println!("Export response status: {}", res.status());

    // Check status
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Export should succeed with status 200"
    );

    // Check content type
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        content_type,
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "Content type should be Excel"
    );

    // Check content disposition
    let content_disposition = res
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok());
    assert!(
        content_disposition.is_some(),
        "Content disposition header should exist"
    );
    assert!(
        content_disposition
            .unwrap()
            .starts_with("attachment; filename="),
        "Content disposition should indicate attachment"
    );
    assert!(
        content_disposition
            .unwrap()
            .contains(&format!("campaign_{}_export_", campaign_id)),
        "Filename should contain campaign ID"
    );
    assert!(
        content_disposition.unwrap().ends_with(".xlsx\""),
        "Filename should end with .xlsx"
    );

    // Get the body and verify it's not empty
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    assert!(!body_bytes.is_empty(), "Excel file should not be empty");

    // Excel files start with PK (ZIP signature)
    assert_eq!(
        &body_bytes[0..2],
        b"PK",
        "File should be a valid Excel file (ZIP format)"
    );

    println!(
        "✅ Export test passed! File size: {} bytes",
        body_bytes.len()
    );
}

#[tokio::test]
async fn test_campaign_export_empty_data() {
    dotenv::dotenv().ok();
    let db = Arc::new(Database::new());

    let app = app(db.clone());

    // Setup user
    let email = format!("empty_export_{}@test.com", uuid::Uuid::new_v4());
    let password = "Password123!";

    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "email": email,
                "username": format!("empty_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]),
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(reg_req).await.unwrap();

    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/auth")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "identifier": email,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(login_res.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = token_json["token"].as_str().unwrap();
    let auth_header = format!("Bearer {}", token);

    // Get config and create campaign
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/platforms")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let platforms: Value = serde_json::from_slice(&body_bytes).unwrap();
    let platform_id = platforms[0]["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/config/platforms/{}/regions", platform_id))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let regions: Value = serde_json::from_slice(&body_bytes).unwrap();
    let region_id = regions[0]["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/ai-models")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let models: Value = serde_json::from_slice(&body_bytes).unwrap();
    let ai_model_id = models[0]["id"].as_i64().unwrap();

    let campaign_payload = serde_json::json!({
        "name": "Empty Export Test",
        "platform_id": platform_id,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": "Test"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/campaigns")
        .header("content-type", "application/json")
        .header("Authorization", &auth_header)
        .body(Body::from(campaign_payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let campaign: Value = serde_json::from_slice(&body_bytes).unwrap();
    let campaign_id = campaign["id"].as_i64().unwrap();

    // Test export with no data - should still succeed
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/campaigns/{}/export", campaign_id))
        .header("Authorization", &auth_header)
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Export should succeed even with no data"
    );

    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    assert!(!body_bytes.is_empty(), "Should return a valid Excel file");
    assert_eq!(&body_bytes[0..2], b"PK", "Should be valid Excel format");

    println!("✅ Empty export test passed!");
}

#[tokio::test]
async fn test_campaign_export_unauthorized() {
    dotenv::dotenv().ok();
    let db = Arc::new(Database::new());

    let app = app(db.clone());

    // Create two users
    let email1 = format!("user1_{}@test.com", uuid::Uuid::new_v4());
    let email2 = format!("user2_{}@test.com", uuid::Uuid::new_v4());
    let password = "Password123!";

    // Register user 1
    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "email": email1,
                "username": format!("u1_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]),
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(reg_req).await.unwrap();

    // Login user 1 and create campaign
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/auth")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "identifier": email1,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(login_res.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token1 = token_json["token"].as_str().unwrap();

    // Get config
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/platforms")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let platforms: Value = serde_json::from_slice(&body_bytes).unwrap();
    let platform_id = platforms[0]["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/config/platforms/{}/regions", platform_id))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let regions: Value = serde_json::from_slice(&body_bytes).unwrap();
    let region_id = regions[0]["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/config/ai-models")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let models: Value = serde_json::from_slice(&body_bytes).unwrap();
    let ai_model_id = models[0]["id"].as_i64().unwrap();

    // Create campaign as user 1
    let campaign_payload = serde_json::json!({
        "name": "User 1 Campaign",
        "platform_id": platform_id,
        "region_id": region_id,
        "ai_model_id": ai_model_id,
        "schedule_type": "ONCE",
        "product_prompt": "Test"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/campaigns")
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token1))
        .body(Body::from(campaign_payload.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
    let campaign: Value = serde_json::from_slice(&body_bytes).unwrap();
    let campaign_id = campaign["id"].as_i64().unwrap();

    // Register and login user 2
    let reg_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "email": email2,
                "username": format!("u2_{}", &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]),
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let _ = app.clone().oneshot(reg_req).await.unwrap();

    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/auth")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "identifier": email2,
                "password": password
            })
            .to_string(),
        ))
        .unwrap();
    let login_res = app.clone().oneshot(login_req).await.unwrap();
    let body_bytes = hyper::body::to_bytes(login_res.into_body()).await.unwrap();
    let token_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token2 = token_json["token"].as_str().unwrap();

    // Try to export user 1's campaign as user 2 - should fail
    let req = Request::builder()
        .method("GET")
        .uri(&format!("/api/v1/campaigns/{}/export", campaign_id))
        .header("Authorization", format!("Bearer {}", token2))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();

    assert!(
        res.status() == StatusCode::FORBIDDEN || res.status() == StatusCode::NOT_FOUND,
        "User 2 should not be able to export user 1's campaign"
    );

    println!("✅ Unauthorized export test passed!");
}
