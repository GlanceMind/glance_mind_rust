use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;
use std::str::FromStr;
use validator::Validate;

/// Campaign status enum for type-safe status handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CampaignStatus {
    /// Initial state when campaign is created but not yet activated
    Draft,
    /// Campaign is running and processing tasks
    Active,
    /// Campaign is temporarily paused by user
    Paused,
    /// Campaign is in the process of stopping (waiting for active tasks to complete)
    Stopping,
    /// Campaign has been stopped by user (graceful stop completed)
    Stopped,
    /// Campaign has completed naturally (reached end date or max count)
    Completed,
    /// Campaign has been archived
    Archived,
}

impl CampaignStatus {
    /// Check if the status allows new tasks to be created
    pub fn allows_new_tasks(&self) -> bool {
        matches!(self, CampaignStatus::Active)
    }

    /// Check if the campaign is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            CampaignStatus::Stopped | CampaignStatus::Completed | CampaignStatus::Archived
        )
    }

    /// All valid status values for validation
    pub const ALL: &'static [CampaignStatus] = &[
        CampaignStatus::Draft,
        CampaignStatus::Active,
        CampaignStatus::Paused,
        CampaignStatus::Stopping,
        CampaignStatus::Stopped,
        CampaignStatus::Completed,
        CampaignStatus::Archived,
    ];
}

impl fmt::Display for CampaignStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CampaignStatus::Draft => write!(f, "DRAFT"),
            CampaignStatus::Active => write!(f, "ACTIVE"),
            CampaignStatus::Paused => write!(f, "PAUSED"),
            CampaignStatus::Stopping => write!(f, "STOPPING"),
            CampaignStatus::Stopped => write!(f, "STOPPED"),
            CampaignStatus::Completed => write!(f, "COMPLETED"),
            CampaignStatus::Archived => write!(f, "ARCHIVED"),
        }
    }
}

impl FromStr for CampaignStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "DRAFT" => Ok(CampaignStatus::Draft),
            "ACTIVE" => Ok(CampaignStatus::Active),
            "PAUSED" => Ok(CampaignStatus::Paused),
            "STOPPING" => Ok(CampaignStatus::Stopping),
            "STOPPED" => Ok(CampaignStatus::Stopped),
            "COMPLETED" => Ok(CampaignStatus::Completed),
            "ARCHIVED" => Ok(CampaignStatus::Archived),
            _ => Err(format!("Invalid campaign status: {}", s)),
        }
    }
}

impl From<CampaignStatus> for String {
    fn from(status: CampaignStatus) -> Self {
        status.to_string()
    }
}

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
    #[serde(default)]
    pub reply_template_ids: Option<Vec<i32>>,
    /// Module D3: when this create originates from confirming an assistant
    /// task-template draft, the draft's id is threaded here so the created
    /// campaign can be linked back to its draft (`gm_campaigns.source_draft_id`).
    #[serde(default)]
    pub source_draft_id: Option<uuid::Uuid>,
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
    #[serde(default)]
    pub reply_template_ids: Option<Vec<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignStatusUpdateDto {
    pub status: CampaignStatus,
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
    pub reply_template_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignLogDto {
    pub id: i32,
    pub campaign_id: i32,
    pub log_level: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
