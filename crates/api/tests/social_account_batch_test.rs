//! Social Account Batch Create Integration Tests
//!
//! Tests the batch account creation flow:
//! 1. Batch create accounts with profile range
//! 2. Verify account count limits (max 100)
//! 3. Verify profile range parsing
//! 4. List and verify created accounts
//!
//! To run these tests:
//! 1. Ensure API server is running
//! 2. Run: cargo test --test social_account_batch_test -- --ignored

use serde_json::json;

const BASE_URL: &str = "http://localhost:8000/api/v1";
const TEST_USERNAME: &str = "jacksoom";
const TEST_PASSWORD: &str = "Lifeng941010";

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

/// Test: Batch create accounts with valid profile range (10 accounts)
#[tokio::test]
#[ignore] // Requires running server
async fn test_batch_create_accounts_small_range() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "test_batch",
            "device_id": "device_test_001",
            "profile_start": "account_1",
            "profile_end": "account_10",
            "daily_max_replies": 30
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    
    assert_eq!(body["data"]["created_count"].as_i64(), Some(10));
    assert_eq!(body["data"]["total_attempted"].as_i64(), Some(10));
    assert!(body["data"]["created_ids"].is_array());
    assert_eq!(body["data"]["created_ids"].as_array().unwrap().len(), 10);
    
    println!("✅ Batch created 10 accounts successfully");
    println!("   Created IDs: {:?}", body["data"]["created_ids"]);
}

/// Test: Batch create accounts with maximum limit (100 accounts)
#[tokio::test]
#[ignore]
async fn test_batch_create_accounts_max_limit() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "max_batch",
            "device_id": "device_max_001",
            "profile_start": "acc_1",
            "profile_end": "acc_100",
            "daily_max_replies": 50
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    
    assert_eq!(body["data"]["created_count"].as_i64(), Some(100));
    assert_eq!(body["data"]["total_attempted"].as_i64(), Some(100));
    
    println!("✅ Batch created 100 accounts (max limit) successfully");
}

/// Test: Batch create accounts exceeding max limit (should fail)
#[tokio::test]
#[ignore]
async fn test_batch_create_accounts_exceed_limit() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "exceed_batch",
            "device_id": "device_exceed_001",
            "profile_start": "test_1",
            "profile_end": "test_101",  // 101 accounts - exceeds limit
            "daily_max_replies": 50
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should return 400 Bad Request
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    
    // Verify error message mentions the limit (check both 'message' and 'msg' fields)
    let message = body["message"].as_str()
        .or(body["msg"].as_str())
        .unwrap_or("");
    assert!(message.contains("100") || message.to_lowercase().contains("maximum"));
    
    println!("✅ Correctly rejected batch creation exceeding 100 accounts");
    println!("   Error message: {}", message);
}

/// Test: Batch create accounts with invalid profile range (start > end)
#[tokio::test]
#[ignore]
async fn test_batch_create_accounts_invalid_range() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "invalid_batch",
            "device_id": "device_invalid_001",
            "profile_start": "acc_100",
            "profile_end": "acc_1",  // End is less than start
            "daily_max_replies": 50
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should return 400 Bad Request
    assert_eq!(response.status(), 400);
    
    println!("✅ Correctly rejected invalid profile range (start > end)");
}

/// Test: Batch create accounts with mismatched prefixes
#[tokio::test]
#[ignore]
async fn test_batch_create_accounts_mismatched_prefix() {
    let token = login().await.expect("Failed to login");
    
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "mismatch_batch",
            "device_id": "device_mismatch_001",
            "profile_start": "account_1",
            "profile_end": "profile_10",  // Different prefix
            "daily_max_replies": 50
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Should return 400 Bad Request
    assert_eq!(response.status(), 400);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    
    // Check both 'message' and 'msg' fields
    let message = body["message"].as_str()
        .or(body["msg"].as_str())
        .unwrap_or("");
    assert!(message.to_lowercase().contains("prefix"));
    
    println!("✅ Correctly rejected mismatched profile prefixes");
    println!("   Error message: {}", message);
}

/// Test: Batch create accounts with optional group_id
#[tokio::test]
#[ignore]
async fn test_batch_create_accounts_with_group() {
    let token = login().await.expect("Failed to login");
    let client = reqwest::Client::new();

    // First, create a group or get an existing group
    let group_response = client
        .get(&format!("{}/social-groups?page=1&page_size=1", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to list groups");

    let group_body: serde_json::Value = group_response.json().await.expect("Failed to parse group response");
    let groups = group_body["data"]["list"].as_array();
    
    let group_id = if let Some(groups) = groups {
        if !groups.is_empty() {
            groups[0]["id"].as_i64().map(|id| id as i32)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(gid) = group_id {
        // Create accounts with group
        let response = client
            .post(&format!("{}/accounts/batch", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "platform_id": 1,
                "username": "grouped_batch",
                "device_id": "device_group_001",
                "profile_start": "grp_1",
                "profile_end": "grp_5",
                "daily_max_replies": 40,
                "group_id": gid
            }))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.expect("Failed to parse response");
        assert_eq!(body["data"]["created_count"].as_i64(), Some(5));
        
        println!("✅ Batch created 5 accounts with group_id={}", gid);
    } else {
        println!("⚠️  No groups found, skipping group assignment test");
    }
}

/// Test: Verify usernames are correctly generated
#[tokio::test]
#[ignore]
async fn test_batch_create_verify_usernames() {
    let token = login().await.expect("Failed to login");
    let client = reqwest::Client::new();

    // Create 3 accounts for easy verification
    let response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "myuser",
            "device_id": "device_verify_001",
            "profile_start": "profile_1",
            "profile_end": "profile_3",
            "daily_max_replies": 25
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    
    let created_ids = body["data"]["created_ids"].as_array().expect("Created IDs not found");
    assert_eq!(created_ids.len(), 3);

    // Verify each created account
    for (i, id_value) in created_ids.iter().enumerate() {
        let account_id = id_value.as_i64().expect("Invalid ID");
        
        // Get account details (assuming there's a detail endpoint or list with filter)
        let list_response = client
            .get(&format!("{}/accounts?page=1&page_size=100", BASE_URL))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Failed to list accounts");

        let list_body: serde_json::Value = list_response.json().await.expect("Failed to parse list response");
        let accounts = list_body["data"]["list"].as_array().expect("List not found");
        
        // Find the created account
        let account = accounts.iter().find(|a| a["id"].as_i64() == Some(account_id));
        if let Some(acc) = account {
            let expected_profile = format!("profile_{}", i + 1);
            let expected_username = format!("myuser_{}", expected_profile);
            
            assert_eq!(acc["profile_name"].as_str(), Some(expected_profile.as_str()));
            assert_eq!(acc["username"].as_str(), Some(expected_username.as_str()));
            
            println!("   Account {}: username={}, profile={}", 
                account_id, 
                acc["username"].as_str().unwrap_or(""),
                acc["profile_name"].as_str().unwrap_or("")
            );
        }
    }

    println!("✅ Verified username generation pattern: username_profile_N");
}

/// Test: Complete batch create flow
#[tokio::test]
#[ignore]
async fn test_batch_create_complete_flow() {
    let token = login().await.expect("Failed to login");
    let client = reqwest::Client::new();

    println!("Step 1: Get initial account count...");
    let initial_stats = client
        .get(&format!("{}/accounts/statistics", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get stats");
    
    let initial_body: serde_json::Value = initial_stats.json().await.expect("Failed to parse stats");
    let initial_total = initial_body["data"]["total"].as_i64().unwrap_or(0);
    println!("   Initial total: {}", initial_total);

    println!("Step 2: Batch create 5 accounts...");
    let create_response = client
        .post(&format!("{}/accounts/batch", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "platform_id": 1,
            "username": "flow_test",
            "device_id": "device_flow_001",
            "profile_start": "flow_1",
            "profile_end": "flow_5",
            "daily_max_replies": 35
        }))
        .send()
        .await
        .expect("Failed to batch create");

    assert_eq!(create_response.status(), 200);
    let create_body: serde_json::Value = create_response.json().await.expect("Failed to parse create response");
    let created_ids = create_body["data"]["created_ids"].as_array().expect("Created IDs not found");
    println!("   Created {} accounts: {:?}", created_ids.len(), created_ids);

    println!("Step 3: Verify new account count...");
    let final_stats = client
        .get(&format!("{}/accounts/statistics", BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get final stats");
    
    let final_body: serde_json::Value = final_stats.json().await.expect("Failed to parse final stats");
    let final_total = final_body["data"]["total"].as_i64().unwrap_or(0);
    println!("   Final total: {}", final_total);

    assert_eq!(final_total, initial_total + 5);
    println!("   ✅ Account count increased by 5");

    println!("Step 4: Cleanup - delete created accounts...");
    for id_value in created_ids {
        let account_id = id_value.as_i64().expect("Invalid ID");
        let delete_response = client
            .delete(&format!("{}/accounts/{}", BASE_URL, account_id))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .expect("Failed to delete account");
        
        assert_eq!(delete_response.status(), 200);
    }
    println!("   ✅ Cleaned up {} accounts", created_ids.len());

    println!("✅ Complete batch create flow test passed!");
}
