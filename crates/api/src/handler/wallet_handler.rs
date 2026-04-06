use crate::dto::wallet_dto::RechargeRequestDto;
use crate::error::api_error::ApiError;
use crate::service::xunhupay_client::PayNotification;
use crate::state::user_state::UserState;
use crate::{api_result, response::ApiResult};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
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

// ======================== XunhuPay endpoints ========================

/// POST /wallet/recharge
/// Authenticated user creates a recharge order via XunhuPay.
pub async fn create_recharge(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    axum::Json(dto): axum::Json<RechargeRequestDto>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let resp = state.wallet_service.create_recharge(user.id, dto).await?;
    Ok(api_result!(resp))
}

/// GET /wallet/recharge/:order_no
/// Poll recharge order status.
pub async fn get_recharge_status(
    Extension(user): Extension<User>,
    Extension(state): Extension<UserState>,
    Path(order_no): Path<String>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let resp = state
        .wallet_service
        .get_recharge_status(&order_no, user.id)
        .await?;
    Ok(api_result!(resp))
}

/// POST /public/payment/notify
/// XunhuPay async callback — no JWT required, signature verified by SDK.
/// Must return plain text "success" on success.
pub async fn payment_notify(
    State(state): State<UserState>,
    axum::Form(notification): axum::Form<PayNotification>,
) -> impl IntoResponse {
    tracing::info!(
        "Payment notify received: order={}, status={}",
        notification.trade_order_id,
        notification.status
    );

    match state
        .wallet_service
        .handle_payment_notify(notification)
        .await
    {
        Ok(()) => "success".to_string(),
        Err(e) => {
            tracing::error!("Payment notify handling failed: {:?}", e);
            "fail".to_string()
        }
    }
}
