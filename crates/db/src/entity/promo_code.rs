use crate::schema::gm_promo_codes;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_promo_codes)]
pub struct PromoCode {
    pub id: i32,
    pub code: String,
    pub points: i32,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub used_by_user_id: Option<i32>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = gm_promo_codes)]
pub struct NewPromoCode {
    pub code: String,
    pub points: i32,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
}
