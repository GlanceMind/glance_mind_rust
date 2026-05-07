use crate::schema::gm_campaigns;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = gm_campaigns)]
pub struct Campaign {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub status: String,
    pub platform_id: i32,
    pub region_id: i32,
    pub ai_model_id: i32,
    pub target_audience: Option<String>,
    pub product_prompt: String,
    pub schedule_config: Option<JsonValue>,
    pub enable_ai_refactor: Option<bool>,
    pub persona_id: Option<i32>,
    pub max_scan_count: Option<i32>,
    pub budget_cap: Option<BigDecimal>,
    pub end_date: Option<DateTime<Utc>>,
    pub schedule_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub keyword: Option<String>,
    pub social_group_id: Option<i32>,
    pub call_to_action: Option<String>,
    pub tone_of_voice: Option<String>,
    pub additional_info: Option<String>,
    pub total_scanned: i32,
    pub auto_like: bool,
    pub auto_follow: bool,
    pub auto_dm: bool,
    // Fields must match schema column order exactly
    pub pending_consumption: BigDecimal,
    pub actual_consumption: BigDecimal,
    pub is_frozen: bool,
    pub search_options: Option<JsonValue>,
    pub auto_reply_comments: bool,
    pub auto_reply_post: bool,
    pub completed_reason: Option<String>,
    pub reply_template_ids: Vec<i32>,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = gm_campaigns)]
pub struct NewCampaign {
    pub user_id: i32,
    pub name: String,
    pub status: Option<String>,
    pub target_audience: Option<String>,
    pub keyword: Option<String>,
    pub schedule_config: Option<JsonValue>,
    pub platform_id: i32,
    pub region_id: i32,
    pub enable_ai_refactor: Option<bool>,
    pub ai_model_id: i32,
    pub persona_id: Option<i32>,
    pub max_scan_count: Option<i32>,
    pub budget_cap: Option<BigDecimal>,
    pub end_date: Option<DateTime<Utc>>,
    pub schedule_type: String,
    pub product_prompt: String,
    pub call_to_action: Option<String>,
    pub tone_of_voice: Option<String>,
    pub additional_info: Option<String>,
    pub social_group_id: Option<i32>,
    pub auto_like: Option<bool>,
    pub auto_follow: Option<bool>,
    pub auto_dm: Option<bool>,
    pub auto_reply_comments: Option<bool>,
    pub auto_reply_post: Option<bool>,
    pub search_options: Option<JsonValue>,
    pub reply_template_ids: Vec<i32>,
}
