use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use validator::Validate;

// Account List Request DTO (dedicated query params for account listing)
#[derive(Debug, Deserialize)]
pub struct AccountListRequest {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    /// Optional group_id filter
    pub group_id: Option<i32>,
    /// Filter by username (partial match, case-insensitive)
    pub username: Option<String>,
    /// Filter by platform_id (exact match)
    pub platform_id: Option<i32>,
    /// Filter by status (exact match, e.g. ACTIVE, RISK_CONTROL, UNAVAILABLE)
    pub status: Option<String>,
    /// Filter by device_id / uuid (partial match, case-insensitive)
    pub device_id: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    10
}

// Social Group DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct SocialGroupDto {
    pub id: i32,
    pub user_id: i32,
    pub platform_id: i32,
    pub group_name: String,
    pub accounts: Option<Vec<SocialAccountDto>>,
    /// Total number of accounts in this group
    pub account_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocialGroupDto {
    pub platform_id: i32,
    pub group_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSocialGroupDto {
    pub group_name: String,
}

// Social Account DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSocialAccountDto {
    /// Platform ID (1=reddit, 2=tiktok, 3=facebook, etc.)
    /// Defaults to 2 (TikTok) if not provided
    #[serde(default = "default_platform_id")]
    pub platform_id: i32,
    pub username: String,
    #[serde(default)]
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    #[validate(range(min = 1, max = 10000))]
    pub daily_max_replies: Option<i32>,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub profile_name: Option<String>,
}

fn default_platform_id() -> i32 {
    2 // Default to TikTok
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSocialAccountDto {
    pub username: Option<String>,
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub status: Option<String>,
    pub group_id: Option<i32>,
    #[validate(range(min = 1, max = 10000))]
    pub daily_max_replies: Option<i32>,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SocialAccountDto {
    pub id: i32,
    pub platform_id: i32,
    pub group_id: Option<i32>,
    pub username: String,
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub status: String,
    pub health_score: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub daily_max_replies: i32,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
}

// Account Statistics DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountStatisticsDto {
    pub total: i64,
    pub active: i64,
    pub risk_control: i64,
    pub unavailable: i64,
}

// Batch Create Social Accounts DTO
#[derive(Debug, Deserialize)]
pub struct BatchCreateAccountsDto {
    /// Platform ID
    pub platform_id: i32,
    /// Base username (will be combined with profile name)
    pub username: String,
    /// Device ID (shared for all accounts)
    pub device_id: Option<String>,
    /// Profile range start (e.g., "account_1")
    pub profile_start: String,
    /// Profile range end (e.g., "account_100")
    pub profile_end: String,
    /// Daily max replies for each account
    #[serde(default = "default_daily_max_replies")]
    pub daily_max_replies: i32,
    /// Group ID (optional)
    pub group_id: Option<i32>,
}

fn default_daily_max_replies() -> i32 {
    50
}

// Batch Create Response DTO
#[derive(Debug, Serialize)]
pub struct BatchCreateResultDto {
    /// Number of accounts successfully created
    pub created_count: i32,
    /// Total accounts attempted
    pub total_attempted: i32,
    /// List of created account IDs
    pub created_ids: Vec<i32>,
    /// Errors if any
    pub errors: Vec<String>,
}

// Batch add accounts to group DTO
#[derive(Debug, Deserialize)]
pub struct BatchAddAccountsToGroupDto {
    pub group_id: i32,
    pub profile_names: Vec<String>, // List of profile names to add
}
