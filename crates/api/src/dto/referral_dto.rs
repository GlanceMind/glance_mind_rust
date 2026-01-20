use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReferralStatsDto {
    pub invite_code: String,
    pub invite_url: String,
    pub total_referrals: i64,
    pub active_referrals: i64,
    pub total_earned: BigDecimal,
    pub commission_rate: BigDecimal,
}
