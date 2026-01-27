use axum::extract::{Path, Query, State};

use crate::dto::video_case_dto::{VideoCaseDetail, VideoCaseListQuery, VideoCaseListResponse};
use crate::error::api_error::ApiError;
use crate::response::api_result::ApiResult;
use crate::state::user_state::UserState;

/// List video cases with pagination and optional filters
/// GET /api/v1/video-cases
/// Query params: page, page_size, status, video_status, category_id, user_id
pub async fn list_video_cases(
    State(state): State<UserState>,
    Query(query): Query<VideoCaseListQuery>,
) -> Result<ApiResult<VideoCaseListResponse>, ApiError> {
    let result = state.video_case_service.list(query).await?;
    Ok(ApiResult::ok(result))
}

/// Get video case detail by video_id
/// GET /api/v1/video-cases/:video_id
pub async fn get_video_case_detail(
    State(state): State<UserState>,
    Path(video_id): Path<i64>,
) -> Result<ApiResult<VideoCaseDetail>, ApiError> {
    let result = state.video_case_service.get_by_video_id(video_id).await?;
    Ok(ApiResult::ok(result))
}

/// Get video case detail by task number
/// GET /api/v1/video-cases/task/:task_no
pub async fn get_video_case_by_task_no(
    State(state): State<UserState>,
    Path(task_no): Path<String>,
) -> Result<ApiResult<VideoCaseDetail>, ApiError> {
    let result = state.video_case_service.get_by_task_no(&task_no).await?;
    Ok(ApiResult::ok(result))
}
