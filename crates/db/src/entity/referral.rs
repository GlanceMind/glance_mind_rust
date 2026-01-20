use crate::schema::{gm_referral_earnings, gm_referrals};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_referrals)]
pub struct Referral {
    pub id: i32,
    pub referrer_id: i32,
    pub referee_id: i32,
    pub commission_rate: bigdecimal::BigDecimal,
    pub total_earned: bigdecimal::BigDecimal,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = gm_referrals)]
pub struct NewReferral {
    pub referrer_id: i32,
    pub referee_id: i32,
    pub commission_rate: bigdecimal::BigDecimal,
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug)]
#[diesel(table_name = gm_referral_earnings)]
pub struct ReferralEarning {
    pub id: i32,
    pub referral_id: i32,
    pub transaction_id: Option<i32>,
    pub amount: bigdecimal::BigDecimal,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = gm_referral_earnings)]
pub struct NewReferralEarning {
    pub referral_id: i32,
    pub transaction_id: Option<i32>,
    pub amount: bigdecimal::BigDecimal,
    pub description: Option<String>,
}
