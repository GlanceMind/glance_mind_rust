use crate::api_ok;
use crate::dto::social_account_dto::{
    BatchCreateAccountsDto, CreateSocialAccountDto, UpdateSocialAccountDto,
};
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{
    extract::{Path, Query},
    response::IntoResponse,
    Extension, Json,
};
use glance_mind_db::entity::user::User;
use validator::Validate;

pub async fn list_accounts(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(req): Query<crate::dto::social_account_dto::AccountListRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let response = state
        .social_account_service
        .list_accounts(user.id, req)
        .await?;
    Ok(api_ok!(response))
}

pub async fn create_account(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Json(dto): Json<CreateSocialAccountDto>,
) -> Result<impl IntoResponse, ApiError> {
    dto.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation error: {e}")))?;
    let account = state
        .social_account_service
        .create_account(user.id, dto)
        .await?;
    Ok(api_ok!(account))
}

pub async fn update_account(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
    Json(dto): Json<UpdateSocialAccountDto>,
) -> Result<impl IntoResponse, ApiError> {
    dto.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation error: {e}")))?;
    let account = state
        .social_account_service
        .update_account(id, user.id, dto)
        .await?;
    Ok(api_ok!(account))
}

pub async fn verify_account(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .social_account_service
        .verify_account(id, user.id)
        .await?;
    Ok(api_ok!(msg: "Account verified", "Account verified"))
}

pub async fn delete_account(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .social_account_service
        .delete_account(id, user.id)
        .await?;
    Ok(api_ok!(msg: "Account deleted", "Account deleted"))
}

pub async fn get_account_statistics(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    let group_id = params.get("group_id").and_then(|s| s.parse::<i32>().ok());

    let stats = state
        .social_account_service
        .get_statistics(user.id, group_id)
        .await?;

    Ok(api_ok!(stats))
}

/// Batch create social accounts
/// POST /accounts/batch
pub async fn batch_create_accounts(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Json(dto): Json<BatchCreateAccountsDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .social_account_service
        .batch_create_accounts(user.id, dto)
        .await?;
    Ok(api_ok!(result))
}

/// Remove account from group (set group_id to NULL)
/// POST /accounts/:id/remove-from-group
pub async fn remove_from_group(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .social_account_service
        .remove_from_group(id, user.id)
        .await?;
    Ok(api_ok!(msg: "Account removed from group", "账户已从分组中移除"))
}
