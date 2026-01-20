use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CampaignCreateDto {
    pub name: String,
    pub platform_id: i32,
    pub region_id: i32,
    pub ai_model_id: i32,
    #[serde(default)]
    pub target_audience: Option<String>,
    #[serde(default)]
    pub enable_ai_refactor: Option<bool>,
    #[serde(default)]
    pub persona_id: Option<i32>,
    #[serde(default)]
    pub max_scan_count: Option<i32>,
    #[serde(default)]
    pub budget_cap: Option<BigDecimal>,
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
    pub schedule_type: String, // INTERVAL, CRON, ONCE
    #[serde(default)]
    pub schedule_config: Option<JsonValue>,
    pub product_prompt: String,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub call_to_action: Option<String>,
    #[serde(default)]
    pub tone_of_voice: Option<String>,
    #[serde(default)]
    pub additional_info: Option<String>,
    pub social_group_id: Option<i32>,
    #[serde(default)]
    pub auto_like: Option<bool>,
    #[serde(default)]
    pub auto_follow: Option<bool>,
    #[serde(default)]
    pub auto_dm: Option<bool>,
    #[serde(default)]
    pub auto_reply_comments: Option<bool>,
    #[serde(default)]
    pub auto_reply_post: Option<bool>,
    #[serde(default)]
    pub search_options: Option<JsonValue>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CampaignUpdateDto {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub target_audience: Option<String>,
    pub status: Option<String>,
    pub platform_id: Option<i32>,
    pub region_id: Option<i32>,
    pub ai_model_id: Option<i32>,
    pub budget_cap: Option<BigDecimal>,
    #[serde(default)]
    pub schedule_config: Option<JsonValue>,
    pub schedule_type: Option<String>,
    pub max_scan_count: Option<i32>,
    pub end_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub product_prompt: Option<String>,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub call_to_action: Option<String>,
    #[serde(default)]
    pub tone_of_voice: Option<String>,
    #[serde(default)]
    pub additional_info: Option<String>,
    pub social_group_id: Option<i32>,
    pub enable_ai_refactor: Option<bool>,
    #[serde(default)]
    pub auto_like: Option<bool>,
    #[serde(default)]
    pub auto_follow: Option<bool>,
    #[serde(default)]
    pub auto_dm: Option<bool>,
    #[serde(default)]
    pub auto_reply_comments: Option<bool>,
    #[serde(default)]
    pub auto_reply_post: Option<bool>,
    #[serde(default)]
    pub search_options: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignStatusUpdateDto {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignStatsDto {
    pub scans: i32,
    pub replies: i32,
    pub conversions: i32,
}

#[derive(Debug, Serialize)]
pub struct CampaignReadDto {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub status: String,
    pub platform_id: i32,
    pub region_id: i32,
    pub ai_model_id: i32,
    pub target_audience: Option<String>,
    pub enable_ai_refactor: Option<bool>,
    pub persona_id: Option<i32>,
    pub max_scan_count: Option<i32>,
    pub budget_cap: Option<BigDecimal>,
    pub actual_consumption: BigDecimal,
    pub end_date: Option<DateTime<Utc>>,
    pub schedule_type: String,
    pub schedule_config: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub product_prompt: String,
    pub keyword: Option<String>,
    pub call_to_action: Option<String>,
    pub tone_of_voice: Option<String>,
    pub additional_info: Option<String>,
    pub social_group_id: Option<i32>,
    pub stats: CampaignStatsDto,
    pub auto_like: bool,
    pub auto_follow: bool,
    pub auto_dm: bool,
    pub auto_reply_comments: bool,
    pub auto_reply_post: bool,
    pub search_options: Option<JsonValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignLogDto {
    pub id: i32,
    pub campaign_id: i32,
    pub log_level: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
