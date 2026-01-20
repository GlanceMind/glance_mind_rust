use crate::dto::user_dto::{UserReadDto, UserUpdatePasswordDto, UserUpdateProfileDto};
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::Extension;
use glance_mind_db::entity::user::User;
use validator::Validate;

pub async fn me(Extension(current_user): Extension<User>) -> ApiResult<impl serde::Serialize> {
    api_result!(UserReadDto::from(current_user))
}

pub async fn update_profile(
    Extension(current_user): Extension<User>,
    Extension(state): Extension<UserState>,
    axum::Json(payload): axum::Json<UserUpdateProfileDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    if payload.validate().is_err() {
        return Err(ApiError::ValidationError(
            "Invalid profile data".to_string(),
        ));
    }

    let updated = state
        .user_service
        .update_profile(current_user.id, payload)
        .await?;
    Ok(api_result!(
        updated,
        "Profile updated successfully",
        "Profile updated successfully"
    ))
}

pub async fn change_password(
    Extension(current_user): Extension<User>,
    Extension(state): Extension<UserState>,
    axum::Json(payload): axum::Json<UserUpdatePasswordDto>,
) -> Result<ApiResult<()>, ApiError> {
    if payload.validate().is_err() {
        return Err(ApiError::ValidationError(
            "Invalid password data".to_string(),
        ));
    }

    state
        .user_service
        .change_password(current_user.id, payload)
        .await?;
    Ok(api_result!(msg: "Password changed successfully", "Password changed successfully"))
}

pub async fn regenerate_api_key(
    Extension(current_user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let new_key = state
        .user_service
        .regenerate_api_key(current_user.id)
        .await?;
    Ok(api_result!(
        new_key,
        "API key regenerated successfully",
        "API key regenerated successfully"
    ))
}

pub async fn get_referral_stats(
    Extension(current_user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://glancemind.org".to_string());

    let stats = state
        .referral_service
        .get_user_stats(current_user.id, &base_url)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    Ok(api_result!(stats))
}
