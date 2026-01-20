use crate::api_ok;
use glance_mind_db::entity::user::User;
use crate::error::api_error::ApiError;
use axum::{response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ScanPostRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ScanPostResponse {
    pub status: String,
    pub message: String,
    pub url: String,
    pub post_count: i32,
}

pub async fn scan_post(
    Extension(user): Extension<User>,
    Json(payload): Json<ScanPostRequest>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!(
        "Scan post request - user_id: {}, url: {}",
        user.id,
        payload.url
    );

    // Placeholder implementation - should actually call crawler service
    let response = ScanPostResponse {
        status: "success".to_string(),
        message: "Post scan completed".to_string(),
        url: payload.url.clone(),
        post_count: 1,
    };

    Ok(api_ok!(response))
}
