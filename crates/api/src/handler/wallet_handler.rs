use crate::dto::wallet_dto::TopUpRequestDto;
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::extract::{Path, Query};
use axum::Extension;
use glance_mind_db::entity::user::User;

pub async fn get_wallet_balance(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let balance = state.wallet_service.get_balance(user.id).await?;
    Ok(api_result!(balance))
}

pub async fn get_wallet_transactions(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Query(req): Query<crate::dto::common::PageRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = state.wallet_service.get_transactions(user.id, req).await?;
    Ok(api_result!(response))
}

pub async fn create_recharge_order(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    axum::Json(dto): axum::Json<TopUpRequestDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let order = state
        .wallet_service
        .create_recharge_order(user.id, dto)
        .await?;
    Ok(api_result!(
        order,
        "Recharge order created successfully",
        "Recharge order created successfully"
    ))
}

pub async fn get_order_status(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(order_id): Path<i32>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let order = state
        .wallet_service
        .get_order_status(order_id, user.id)
        .await?;
    Ok(api_result!(order))
}
