use axum::{body::Body, http::Request};
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use glance_mind_api::config::parameter;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_account_group_association() {
    parameter::init();
    let db = Arc::new(Database::new());
    let app = app(db.clone());

    // 1. Register & Login
    let email = format!("assoc_test_{}@example.com", uuid::Uuid::new_v4());
    let username = format!(
        "assoc_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..10]
    );
    let register_payload = json!({
        "email": email,
        "username": username,
        "password": "password123"
    });

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/register")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&register_payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_payload = json!({
        "identifier": email,
        "password": "password123"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/auth")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_success());
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let token_res: Value = serde_json::from_slice(&body).unwrap();
    let token = token_res["token"].as_str().unwrap();
    let auth_header = format!("Bearer {}", token);

    // 2. Create Group
    let create_group_payload = json!({
        "platform_id": 1,
        "group_name": "Assoc Group"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/social-groups")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(Body::from(
                    serde_json::to_string(&create_group_payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let group_res: Value = serde_json::from_slice(&body).unwrap();
    let group_id = group_res["id"].as_i64().unwrap();

    // 3. Create Account (No Group)
    let create_account_payload = json!({
        "username": "AssocAccount",
        "cookie": "abc",
        "proxy_url": null
    });
    // Assuming POST /api/v1/accounts creates account.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(Body::from(
                    serde_json::to_string(&create_account_payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let account_res: Value = serde_json::from_slice(&body).unwrap();
    let account_id = account_res["id"].as_i64().unwrap();
    assert!(account_res["group_id"].is_null());

    // 4. Update Account -> Assign to Group
    let update_assign = json!({
        "group_id": group_id
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/accounts/{}", account_id))
                .method("PUT")
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(Body::from(serde_json::to_string(&update_assign).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let updated_res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated_res["group_id"].as_i64(), Some(group_id));

    // 5. Update Account -> Unassign (group_id: 0)
    let update_unassign = json!({
        "group_id": 0
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/accounts/{}", account_id))
                .method("PUT")
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(Body::from(serde_json::to_string(&update_unassign).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success());
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let unassigned_res: Value = serde_json::from_slice(&body).unwrap();
    assert!(unassigned_res["group_id"].is_null());
}
