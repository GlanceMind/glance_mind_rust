use crate::schema::gm_pricing_rules;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_pricing_rules)]
pub struct PricingRule {
    pub id: i32,
    pub platform_id: Option<i32>,
    pub action_type: String,
    pub cost_points: BigDecimal,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = gm_pricing_rules)]
pub struct NewPricingRule {
    pub action_type: String,
    pub platform_id: Option<i32>,
    pub cost_points: BigDecimal,
}
