use crate::api_ok;
use crate::dto::agent_dto::{DeviceCommentsQuery, UpdateCommentStatusDto};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::state::user_state::UserState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use validator::Validate;

pub async fn get_comments_by_device(
    State(state): State<UserState>,
    Query(query): Query<DeviceCommentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.agent_service.get_comments_by_device(query).await?;
    Ok(api_ok!(result))
}

pub async fn update_comment_status(
    State(state): State<UserState>,
    Json(dto): Json<UpdateCommentStatusDto>,
) -> Result<impl IntoResponse, ApiError> {
    dto.validate()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e.to_string())))?;
    let result = state.agent_service.update_comment_status(dto).await?;
    Ok(api_ok!(result))
}
