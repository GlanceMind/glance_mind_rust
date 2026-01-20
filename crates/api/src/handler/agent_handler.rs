use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::extract::{Path, Query, State};
use serde::Deserialize;

/// Legacy endpoint - only queries TikTok comments
pub async fn get_video_comments(
    State(state): State<UserState>,
    Path(video_id): Path<i32>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = state
        .agent_service
        .get_video_comments(video_id, req)
        .await?;
    Ok(api_result!(response))
}

/// Query params for unified comments endpoint
#[derive(Debug, Deserialize)]
pub struct UnifiedCommentsQuery {
    pub platform_id: i32,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

/// Unified endpoint - queries comments for any platform
pub async fn get_unified_comments(
    State(state): State<UserState>,
    Path(content_db_id): Path<i32>,
    Query(query): Query<UnifiedCommentsQuery>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let req = crate::dto::common::PageRequest {
        page: query.page,
        page_size: query.page_size,
    };
    let response = state
        .agent_service
        .get_unified_comments(content_db_id, query.platform_id, req)
        .await?;
    Ok(api_result!(response))
}
