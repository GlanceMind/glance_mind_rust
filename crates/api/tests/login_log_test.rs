mod common;

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

/// Type alias for login log record tuple (id, user_id, status, ip, user_agent)
type LoginLogRecord = (i32, i32, String, Option<String>, Option<String>);

/// Test login log with direct database user insertion
#[tokio::test]
async fn test_login_log_with_direct_user() {
    dotenv::dotenv().ok();

    let db = Arc::new(Database::new());
    let pool = &db.pool;
    let mut conn = pool.get().expect("Failed to get DB connection");

    // Create test user directly in database (bypass email verification)
    use diesel::prelude::*;
    use glance_mind_api::schema::gm_users;

    let email = format!("directtest_{}@example.com", uuid::Uuid::new_v4());
    let password = "TestPassword123!";
    // Hash password using bcrypt
    let password_hash = bcrypt::hash(password, 4).expect("Failed to hash password");

    // Insert user
    diesel::insert_into(gm_users::table)
        .values((
            gm_users::email.eq(&email),
            gm_users::password_hash.eq(&password_hash),
            gm_users::full_name.eq("Test User"),
            gm_users::role.eq("user"),
            gm_users::is_active.eq(true),
            gm_users::status.eq("active"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert test user");

    // Get inserted user ID
    let user_id: i32 = gm_users::table
        .filter(gm_users::email.eq(&email))
        .select(gm_users::id)
        .first(&mut conn)
        .expect("Failed to get user id");

    println!("Created test user with id: {}", user_id);

    let app = app(db.clone());

    // Login
    let login_payload = serde_json::json!({
        "identifier": email,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.168.1.100")
        .header("user-agent", "DirectTestClient/1.0")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    println!("Login response: {:?}", json);

    assert_eq!(status, StatusCode::OK, "Login should succeed");

    // Verify response format
    common::assert_success(&json);

    // Check database for login log
    use glance_mind_api::schema::gm_login_logs;

    let mut conn = pool.get().expect("Failed to get DB connection");
    let logs: Vec<LoginLogRecord> = gm_login_logs::table
        .select((
            gm_login_logs::id,
            gm_login_logs::user_id,
            gm_login_logs::login_status,
            gm_login_logs::ip_address,
            gm_login_logs::user_agent,
        ))
        .filter(gm_login_logs::user_id.eq(user_id))
        .order(gm_login_logs::id.desc())
        .limit(1)
        .load(&mut conn)
        .expect("Failed to query login logs");

    assert!(
        !logs.is_empty(),
        "Should have at least one login log for user {}",
        user_id
    );
    let (_, log_user_id, status, ip, ua) = &logs[0];
    assert_eq!(*log_user_id, user_id, "User ID should match");
    assert_eq!(status, "SUCCESS", "Login status should be SUCCESS");
    assert_eq!(
        ip.as_deref(),
        Some("192.168.1.100"),
        "IP address should match"
    );
    assert!(
        ua.as_ref()
            .map(|s| s.contains("DirectTestClient"))
            .unwrap_or(false),
        "User agent should contain DirectTestClient"
    );

    println!("✅ Login log test passed!");
}

/// Test login success creates log (using registration flow)
#[tokio::test]
async fn test_login_creates_success_log() {
    dotenv::dotenv().ok();

    let db = Arc::new(Database::new());

    let app = app(db.clone());

    // Create a test user first
    let email = format!("logintest_{}@example.com", uuid::Uuid::new_v4());
    let username = format!("logintest_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let password = "TestPassword123!";

    // Register user
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

    // If registration fails (might require email verification), skip this test
    if response.status() != StatusCode::OK {
        println!("Registration failed (might require email verification), skipping login log test");
        return;
    }

    // Login
    let login_payload = serde_json::json!({
        "identifier": email,
        "password": password
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "192.168.1.1")
        .header("user-agent", "TestClient/1.0")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "Login should succeed");

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    // Verify response format
    common::assert_success(&json);

    // Check database for login log
    use diesel::prelude::*;
    use glance_mind_api::schema::gm_login_logs;

    let pool = &db.pool;
    let mut conn = pool.get().expect("Failed to get DB connection");
    let logs: Vec<(i32, String, Option<String>, Option<String>)> = gm_login_logs::table
        .select((
            gm_login_logs::id,
            gm_login_logs::login_status,
            gm_login_logs::ip_address,
            gm_login_logs::user_agent,
        ))
        .order(gm_login_logs::id.desc())
        .limit(1)
        .load(&mut conn)
        .expect("Failed to query login logs");

    assert!(!logs.is_empty(), "Should have at least one login log");
    let (_, status, ip, ua) = &logs[0];
    assert_eq!(status, "SUCCESS", "Login status should be SUCCESS");
    assert_eq!(
        ip.as_deref(),
        Some("192.168.1.1"),
        "IP address should match"
    );
    assert!(
        ua.as_ref()
            .map(|s| s.contains("TestClient"))
            .unwrap_or(false),
        "User agent should contain TestClient"
    );
}

/// Test login failure creates log (direct user insertion)
#[tokio::test]
async fn test_login_failure_log_with_direct_user() {
    dotenv::dotenv().ok();

    let db = Arc::new(Database::new());
    let pool = &db.pool;
    let mut conn = pool.get().expect("Failed to get DB connection");

    // Create test user directly in database (bypass email verification)
    use diesel::prelude::*;
    use glance_mind_api::schema::gm_users;

    let email = format!("failtest_{}@example.com", uuid::Uuid::new_v4());
    let password = "TestPassword123!";
    let password_hash = bcrypt::hash(password, 4).expect("Failed to hash password");

    // Insert user
    diesel::insert_into(gm_users::table)
        .values((
            gm_users::email.eq(&email),
            gm_users::password_hash.eq(&password_hash),
            gm_users::full_name.eq("Fail Test User"),
            gm_users::role.eq("user"),
            gm_users::is_active.eq(true),
            gm_users::status.eq("active"),
        ))
        .execute(&mut conn)
        .expect("Failed to insert test user");

    // Get inserted user ID
    let user_id: i32 = gm_users::table
        .filter(gm_users::email.eq(&email))
        .select(gm_users::id)
        .first(&mut conn)
        .expect("Failed to get user id");

    println!("Created fail test user with id: {}", user_id);

    let app = app(db.clone());

    // Login with wrong password
    let login_payload = serde_json::json!({
        "identifier": email,
        "password": "WrongPassword123!"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "10.0.0.100")
        .header("user-agent", "FailDirectTestClient/1.0")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    println!("Failed login response: {:?}", json);

    // Login should fail (401 or other error code)
    assert_ne!(
        status,
        StatusCode::OK,
        "Login with wrong password should fail"
    );

    // Check database for failed login log
    use glance_mind_api::schema::gm_login_logs;

    let mut conn = pool.get().expect("Failed to get DB connection");
    let logs: Vec<LoginLogRecord> = gm_login_logs::table
        .select((
            gm_login_logs::id,
            gm_login_logs::user_id,
            gm_login_logs::login_status,
            gm_login_logs::ip_address,
            gm_login_logs::failure_reason,
        ))
        .filter(gm_login_logs::user_id.eq(user_id))
        .filter(gm_login_logs::login_status.eq("FAILED"))
        .order(gm_login_logs::id.desc())
        .limit(1)
        .load(&mut conn)
        .expect("Failed to query login logs");

    assert!(
        !logs.is_empty(),
        "Should have at least one failed login log for user {}",
        user_id
    );
    let (_, log_user_id, log_status, ip, reason) = &logs[0];
    assert_eq!(*log_user_id, user_id, "User ID should match");
    assert_eq!(log_status, "FAILED", "Login status should be FAILED");
    assert_eq!(ip.as_deref(), Some("10.0.0.100"), "IP address should match");
    assert!(reason.is_some(), "Failure reason should be recorded");

    println!("✅ Failed login log test passed!");
}

/// Test login failure creates log (using registration flow)
#[tokio::test]
async fn test_login_creates_failure_log() {
    dotenv::dotenv().ok();

    let db = Arc::new(Database::new());

    let app = app(db.clone());

    // Create a test user first
    let email = format!("failtest_{}@example.com", uuid::Uuid::new_v4());
    let username = format!("failtest_{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let password = "TestPassword123!";

    // Register user
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

    // If registration fails (might require email verification), skip this test
    if response.status() != StatusCode::OK {
        println!("Registration failed (might require email verification), skipping login failure log test");
        return;
    }

    // Login with wrong password
    let login_payload = serde_json::json!({
        "identifier": email,
        "password": "WrongPassword123!"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "10.0.0.1")
        .header("user-agent", "FailTestClient/1.0")
        .body(Body::from(login_payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();

    // Login should fail (401 or other error code)
    assert_ne!(
        response.status(),
        StatusCode::OK,
        "Login with wrong password should fail"
    );

    // Check database for failed login log
    use diesel::prelude::*;
    use glance_mind_api::schema::gm_login_logs;

    let pool = &db.pool;
    let mut conn = pool.get().expect("Failed to get DB connection");
    let logs: Vec<(i32, String, Option<String>, Option<String>)> = gm_login_logs::table
        .select((
            gm_login_logs::id,
            gm_login_logs::login_status,
            gm_login_logs::ip_address,
            gm_login_logs::failure_reason,
        ))
        .filter(gm_login_logs::login_status.eq("FAILED"))
        .order(gm_login_logs::id.desc())
        .limit(1)
        .load(&mut conn)
        .expect("Failed to query login logs");

    assert!(
        !logs.is_empty(),
        "Should have at least one failed login log"
    );
    let (_, status, ip, reason) = &logs[0];
    assert_eq!(status, "FAILED", "Login status should be FAILED");
    assert_eq!(ip.as_deref(), Some("10.0.0.1"), "IP address should match");
    assert!(reason.is_some(), "Failure reason should be recorded");
}
