use crate::api_ok;
use crate::dto::upload_task_dto::{CreateUploadTaskDto, DeviceTaskQueryDto};
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::state::user_state::UserState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use glance_mind_db::entity::user::User;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UploadTaskQueryParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub status: Option<String>,
    pub platform_id: Option<i32>,
}

pub async fn create_upload_task(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(payload): ValidatedRequest<CreateUploadTaskDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .upload_task_service
        .create_upload_task(user.id, payload)
        .await?;
    Ok(api_ok!(result))
}

pub async fn get_tasks_by_device(
    State(state): State<UserState>,
    Query(query): Query<DeviceTaskQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.upload_task_service.get_tasks_by_device(query).await?;
    Ok(api_ok!(result))
}

pub async fn list_my_tasks(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(params): Query<UploadTaskQueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .upload_task_service
        .list_user_tasks_with_filter(
            user.id,
            params.page.unwrap_or(1),
            params.per_page.unwrap_or(10),
            params.status,
            params.platform_id,
        )
        .await?;
    Ok(api_ok!(result))
}

pub async fn update_task_status(
    State(state): State<UserState>,
    Json(dto): Json<crate::dto::upload_task_dto::UpdateTaskStatusDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .upload_task_service
        .update_task_status_public(dto.task_id, dto.status)
        .await?;
    Ok(api_ok!(result))
}
