use crate::api_ok;
use crate::dto::social_account_dto::{CreateSocialGroupDto, UpdateSocialGroupDto};
use glance_mind_db::entity::user::User;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    Json,
};

pub async fn list_groups(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state.social_group_service.list_groups(user.id, req).await?;
    Ok(api_ok!(response))
}

pub async fn create_group(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Json(dto): Json<CreateSocialGroupDto>,
) -> Result<impl IntoResponse, ApiError> {
    let group = state
        .social_group_service
        .create_group(user.id, dto)
        .await?;
    Ok(api_ok!(group))
}

pub async fn update_group(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
    Json(dto): Json<UpdateSocialGroupDto>,
) -> Result<impl IntoResponse, ApiError> {
    let group = state
        .social_group_service
        .update_group(id, user.id, dto)
        .await?;
    Ok(api_ok!(group))
}

pub async fn delete_group(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state.social_group_service.delete_group(id, user.id).await?;
    Ok(api_ok!(msg: "Group deleted", "Group deleted"))
}
