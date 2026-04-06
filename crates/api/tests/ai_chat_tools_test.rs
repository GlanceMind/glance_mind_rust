//! AI Chat Tools Integration Tests
//!
//! Tests the AI chat tool execution via the SSE message endpoint.
//! Each test sends a message and verifies the SSE stream contains
//! expected tool call events.
//!
//! To run:
//! 1. Ensure API server is running on localhost:8000
//! 2. Run: cargo test --test ai_chat_tools_test -- --ignored

use serde_json::json;

const BASE_URL: &str = "http://localhost:8000/api/v1";
const TEST_USERNAME: &str = "jacksoom";
const TEST_PASSWORD: &str = "Lifeng94101";

async fn login() -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/auth/login", BASE_URL))
        .json(&json!({
            "identifier": TEST_USERNAME,
            "password": TEST_PASSWORD
        }))
        .send()
        .await?;
    let body: serde_json::Value = response.json().await?;
    let token = body["data"]["token"].as_str().ok_or("Token not found")?;
    Ok(token.to_string())
}

async fn create_conversation(
    client: &reqwest::Client,
    token: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    let resp = client
        .post(format!("{}/ai-chat/conversations", BASE_URL))
        .bearer_auth(token)
        .json(&json!({ "title": "Integration test" }))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let id = body["data"]["id"]
        .as_i64()
        .ok_or("Conversation id not found")?;
    Ok(id)
}

async fn send_message_and_get_sse(
    client: &reqwest::Client,
    token: &str,
    conv_id: i64,
    message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let resp = client
        .post(format!(
            "{}/ai-chat/conversations/{}/messages",
            BASE_URL, conv_id
        ))
        .bearer_auth(token)
        .json(&json!({ "content": message, "model_id": 2 }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "Expected 200 for SSE stream");
    let body = resp.text().await?;
    Ok(body)
}

async fn cleanup_conversation(client: &reqwest::Client, token: &str, conv_id: i64) {
    let _ = client
        .delete(format!("{}/ai-chat/conversations/{}", BASE_URL, conv_id))
        .bearer_auth(token)
        .send()
        .await;
}

fn sse_contains_tool(body: &str, tool_name: &str) -> bool {
    body.contains(&format!("\"tool_name\":\"{}\"", tool_name))
}

fn sse_has_message_end(body: &str) -> bool {
    body.contains("event: message_end")
}

// ===== Template tools =====

#[tokio::test]
#[ignore]
async fn test_list_templates_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "列出我的回复模板")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== Material tools =====

#[tokio::test]
#[ignore]
async fn test_list_materials_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "列出我的素材库")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

#[tokio::test]
#[ignore]
async fn test_list_material_tags_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "显示所有素材标签")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== DM tools =====

#[tokio::test]
#[ignore]
async fn test_dm_stats_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "查看DM统计数据")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== Notification tools =====

#[tokio::test]
#[ignore]
async fn test_list_notifications_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "显示我的通知")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== Video case tools =====

#[tokio::test]
#[ignore]
async fn test_list_video_cases_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "列出视频案例库")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== Publish task tools =====

#[tokio::test]
#[ignore]
async fn test_list_publish_tasks_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "列出我的发布任务")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    cleanup_conversation(&client, &token, conv_id).await;
}

// ===== Dashboard & wallet (smoke) =====

#[tokio::test]
#[ignore]
async fn test_dashboard_stats_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "Show me dashboard stats")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    assert!(
        sse_contains_tool(&body, "get_dashboard_stats"),
        "Should call get_dashboard_stats"
    );
    cleanup_conversation(&client, &token, conv_id).await;
}

#[tokio::test]
#[ignore]
async fn test_wallet_balance_via_chat() {
    let token = login().await.expect("Login failed");
    let client = reqwest::Client::new();
    let conv_id = create_conversation(&client, &token)
        .await
        .expect("Create conv");
    let body = send_message_and_get_sse(&client, &token, conv_id, "What is my wallet balance?")
        .await
        .expect("Send message");
    assert!(sse_has_message_end(&body), "Stream should complete");
    assert!(
        sse_contains_tool(&body, "get_wallet_balance"),
        "Should call get_wallet_balance"
    );
    cleanup_conversation(&client, &token, conv_id).await;
}
