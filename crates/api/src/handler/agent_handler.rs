use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::extract::{Path, Query, State};

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
