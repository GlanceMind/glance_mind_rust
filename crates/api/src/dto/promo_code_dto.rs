use serde::{Deserialize, Serialize};

/// Promo code redemption request DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemPromoCodeRequest {
    pub code: String,
}

/// Promo code redemption response DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemPromoCodeResponse {
    pub success: bool,
    pub message: String,
    pub points: Option<i32>,
    pub new_balance: Option<i32>,
}
