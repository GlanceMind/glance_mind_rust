use axum::body::Body;
use axum::http::{Request, StatusCode};
use glance_mind_api::app;
use glance_mind_api::config::database::Database;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn test_wallet_route_exists() {
    dotenv::dotenv().ok();
    let db = Arc::new(Database::new());
    let app = app(db);

    // Test without auth first - should get 401 not 404
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/wallet/balance")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    let status = res.status();

    println!("Status: {}", status);
    // Should be 401 (Unauthorized) not 404 (Not Found)
    // 404 means route doesn't exist
    // 401 means route exists but auth failed
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "Route should exist (expected 401, got 404)"
    );
}
