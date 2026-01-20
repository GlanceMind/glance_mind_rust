use crate::dto::promo_code_dto::RedeemPromoCodeRequest;
use crate::error::api_error::ApiError;
use crate::service::promo_code_service::PromoCodeService;
use crate::{api_result, response::ApiResult};
use axum::extract::State;
use axum::Extension;
use glance_mind_db::entity::user::User;
use std::sync::Arc;

/// Redeem promo code
pub async fn redeem_promo_code(
    State(service): State<Arc<PromoCodeService>>,
    Extension(user): Extension<User>,
    axum::Json(request): axum::Json<RedeemPromoCodeRequest>,
) -> Result<ApiResult<impl serde::Serialize>, ApiError> {
    let response = service.redeem_promo_code(user.id, request).await?;
    Ok(api_result!(
        response,
        "Promo code redeemed successfully",
        "Promo code redeemed successfully"
    ))
}
