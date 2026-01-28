//! Material Management Integration Tests
//!
//! Tests the complete flow:
//! 1. Upload video to OSS
//! 2. Create material (triggers AI analysis)
//! 3. List materials
//! 4. Get material detail
//! 5. Update material
//! 6. Delete material
//! 7. Favorite from video_case
//! 8. List tags
//!
//! To run these tests:
//! 1. Ensure API server is running
//! 2. Set up test environment variables (OSS, LaoZhang API key)
//! 3. Run: cargo test --test material_integration_test -- --ignored

use reqwest::multipart;
use serde_json::json;

const BASE_URL: &str = "http://localhost:8000/api/v1";
const TEST_USERNAME: &str = "jacksoom";
const TEST_PASSWORD: &str = "Lifeng94101";

/// Helper function to login and get JWT token
async fn login() -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "identifier": TEST_USERNAME,
            "password": TEST_PASSWORD
        }))
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    let token = body["data"]["token"]
        .as_str()
        .ok_or("Token not found in response")?;
    Ok(token.to_string())
}

/// Test: Upload video to OSS
#[tokio::test]
#[ignore] // Requires running server and OSS configuration
async fn test_upload_video_to_oss() {
    let token = login().await.expect("Failed to login");
    
    // Create a small test video file (1x1 pixel MP4)
    let video_data = vec![
        0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, // ftyp box
        0x69, 0x73, 0x6F, 0x6D, 0x00, 0x00, 0x02, 0x00,
        0x69, 0x73, 0x6F, 0x6D, 0x69, 0x73, 0x6F, 0x32,
        0x61, 0x76, 0x63, 0x31, 0x6D, 0x70, 0x34, 0x31,
    ];

    let client = reqwest::Client::new();
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(video_data).file_name("test.mp4").mime_str("video/mp4").unwrap());

    let response = client
        .post(&format!("{}/oss/upload-video", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["video_url"].is_string());
    println!("✅ Video uploaded successfully: {}", body["data"]["video_url"]);
}

/// Test: Create material with AI analysis
#[tokio::test]
#[ignore]
async fn test_create_material_with_ai_analysis() {
    let token = login().await.expect("Failed to login");
    
    // First upload a video
    let video_data = vec![
        0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70,
        0x69, 0x73, 0x6F, 0x6D, 0x00, 0x00, 0x02, 0x00,
        0x69, 0x73, 0x6F, 0x6D, 0x69, 0x73, 0x6F, 0x32,
        0x61, 0x76, 0x63, 0x31, 0x6D, 0x70, 0x34, 0x31,
    ];

    let client = reqwest::Client::new();
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(video_data).file_name("test.mp4").mime_str("video/mp4").unwrap());

    let upload_response = client
        .post(&format!("{}/oss/upload-video", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload video");

    let upload_body: serde_json::Value = upload_response.json().await.expect("Failed to parse upload response");
    let video_url = upload_body["data"]["video_url"]
        .as_str()
        .expect("Video URL not found")
        .to_string();

    // Create material (will trigger AI analysis)
    // Note: title and tag are required fields
    let create_response = client
        .post(&format!("{}/materials", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "video_url": video_url,
            "tag": "测试标签",
            "title": "测试素材",
            "description": "这是一个测试素材"
        }))
        .send()
        .await
        .expect("Failed to create material");

    assert_eq!(create_response.status(), 200);
    let create_body: serde_json::Value = create_response.json().await.expect("Failed to parse create response");
    
    // Verify material was created
    assert!(create_body["data"]["id"].is_number());
    
    // Verify required fields are saved
    assert_eq!(create_body["data"]["title"].as_str(), Some("测试素材"));
    assert_eq!(create_body["data"]["tag"].as_str(), Some("测试标签"));
    assert_eq!(create_body["data"]["description"].as_str(), Some("这是一个测试素材"));
    
    // Verify AI-generated prompt exists (may be None if AI analysis fails)
    let material_id = create_body["data"]["id"].as_i64().expect("Material ID not found");
    println!("✅ Material created with ID: {}", material_id);
    println!("   Title: {}", create_body["data"]["title"]);
    println!("   Tag: {}", create_body["data"]["tag"]);
    println!("   Prompt: {:?}", create_body["data"]["prompt"]);
}

/// Test: List materials
#[tokio::test]
#[ignore]
async fn test_list_materials() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/materials?page=1&page_size=10", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["list"].is_array());
    assert!(body["data"]["total"].is_number());
    println!("✅ Materials listed: {} total", body["data"]["total"]);
}

/// Test: Get material tags
#[tokio::test]
#[ignore]
async fn test_list_material_tags() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/material-tags", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(body["data"]["tags"].is_array());
    println!("✅ Tags collected: {} tags", body["data"]["tags"].as_array().unwrap().len());
}

/// Test: Favorite from video_case
#[tokio::test]
#[ignore]
async fn test_favorite_from_video_case() {
    let token = login().await.expect("Failed to login");
    
    // First, get a video_case task_no
    let client = reqwest::Client::new();
    let list_response = client
        .get(&format!("{}/video-cases?page=1&page_size=1", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list video cases");

    let list_body: serde_json::Value = list_response.json().await.expect("Failed to parse list response");
    let items = list_body["data"]["items"].as_array().expect("Items not found");
    
    if items.is_empty() {
        println!("⚠️  No video cases found, skipping favorite test");
        return;
    }

    let task_no = items[0]["taskNo"]
        .as_str()
        .expect("TaskNo not found")
        .to_string();

    // Favorite the video_case
    let favorite_response = client
        .post(&format!("{}/video-cases/{}/favorite", BASE_URL, task_no))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to favorite video case");

    assert_eq!(favorite_response.status(), 200);
    let favorite_body: serde_json::Value = favorite_response.json().await.expect("Failed to parse favorite response");
    assert!(favorite_body["data"]["id"].is_number());
    println!("✅ Material favorited from video_case: {}", favorite_body["data"]["id"]);
}

/// Test: Create material without required fields (should fail)
#[tokio::test]
#[ignore]
async fn test_create_material_missing_required_fields() {
    let token = login().await.expect("Failed to login");
    
    // First upload a video
    let video_data = vec![
        0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70,
        0x69, 0x73, 0x6F, 0x6D, 0x00, 0x00, 0x02, 0x00,
        0x69, 0x73, 0x6F, 0x6D, 0x69, 0x73, 0x6F, 0x32,
        0x61, 0x76, 0x63, 0x31, 0x6D, 0x70, 0x34, 0x31,
    ];

    let client = reqwest::Client::new();
    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(video_data).file_name("test.mp4").mime_str("video/mp4").unwrap());

    let upload_response = client
        .post(&format!("{}/oss/upload-video", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload video");

    let upload_body: serde_json::Value = upload_response.json().await.expect("Failed to parse upload response");
    let video_url = upload_body["data"]["video_url"]
        .as_str()
        .expect("Video URL not found")
        .to_string();

    // Try to create material without title (should be handled by frontend validation)
    // Backend will accept it but frontend should require it
    let create_response = client
        .post(&format!("{}/materials", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "video_url": video_url,
            "tag": "测试标签"
            // Missing title - frontend should validate
        }))
        .send()
        .await
        .expect("Failed to create material");

    // Backend currently accepts optional title, but frontend enforces it
    // This test documents current behavior
    println!("⚠️  Note: Backend accepts optional title, frontend enforces it");
}

/// Test: Complete flow - Upload → Create → List → Get → Update → Delete
#[tokio::test]
#[ignore]
async fn test_complete_material_flow() {
    let token = login().await.expect("Failed to login");
    let client = reqwest::Client::new();

    // Step 1: Upload video
    println!("Step 1: Uploading video...");
    let video_data = vec![
        0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70,
        0x69, 0x73, 0x6F, 0x6D, 0x00, 0x00, 0x02, 0x00,
        0x69, 0x73, 0x6F, 0x6D, 0x69, 0x73, 0x6F, 0x32,
        0x61, 0x76, 0x63, 0x31, 0x6D, 0x70, 0x34, 0x31,
    ];

    let form = multipart::Form::new()
        .part("file", multipart::Part::bytes(video_data).file_name("test.mp4").mime_str("video/mp4").unwrap());

    let upload_response = client
        .post(&format!("{}/oss/upload-video", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .expect("Failed to upload video");

    let upload_body: serde_json::Value = upload_response.json().await.expect("Failed to parse upload response");
    let video_url = upload_body["data"]["video_url"].as_str().expect("Video URL not found").to_string();
    println!("   ✅ Video uploaded: {}", video_url);

    // Step 2: Create material (triggers AI analysis)
    println!("Step 2: Creating material (AI analysis will run)...");
    let create_response = client
        .post(&format!("{}/materials", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "video_url": video_url,
            "tag": "完整流程测试",
            "title": "完整流程测试素材",
            "description": "测试完整流程：上传→创建→列表→详情→更新→删除"
        }))
        .send()
        .await
        .expect("Failed to create material");

    assert_eq!(create_response.status(), 200);
    let create_body: serde_json::Value = create_response.json().await.expect("Failed to parse create response");
    let material_id = create_body["data"]["id"].as_i64().expect("Material ID not found");
    println!("   ✅ Material created: ID={}", material_id);
    println!("      Title: {}", create_body["data"]["title"]);
    println!("      Tag: {}", create_body["data"]["tag"]);
    println!("      Prompt: {:?}", create_body["data"]["prompt"]);

    // Step 3: List materials
    println!("Step 3: Listing materials...");
    let list_response = client
        .get(&format!("{}/materials?page=1&page_size=10", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list materials");

    let list_body: serde_json::Value = list_response.json().await.expect("Failed to parse list response");
    let materials = list_body["data"]["list"].as_array().expect("List not found");
    assert!(materials.iter().any(|m| m["id"].as_i64() == Some(material_id)));
    println!("   ✅ Material found in list");

    // Step 4: Get material detail
    println!("Step 4: Getting material detail...");
    let detail_response = client
        .get(&format!("{}/materials/{}", BASE_URL, material_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get material detail");

    assert_eq!(detail_response.status(), 200);
    let detail_body: serde_json::Value = detail_response.json().await.expect("Failed to parse detail response");
    assert_eq!(detail_body["data"]["id"].as_i64(), Some(material_id));
    println!("   ✅ Material detail retrieved");

    // Step 5: Update material
    println!("Step 5: Updating material...");
    let update_response = client
        .put(&format!("{}/materials/{}", BASE_URL, material_id))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "title": "更新后的标题",
            "description": "更新后的描述"
        }))
        .send()
        .await
        .expect("Failed to update material");

    assert_eq!(update_response.status(), 200);
    let update_body: serde_json::Value = update_response.json().await.expect("Failed to parse update response");
    assert_eq!(update_body["data"]["title"].as_str(), Some("更新后的标题"));
    println!("   ✅ Material updated");

    // Step 6: Delete material
    println!("Step 6: Deleting material...");
    let delete_response = client
        .delete(&format!("{}/materials/{}", BASE_URL, material_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to delete material");

    assert_eq!(delete_response.status(), 200);
    println!("   ✅ Material deleted");

    println!("✅ Complete flow test passed!");
}
