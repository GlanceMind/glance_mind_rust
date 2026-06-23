use super::fb_page::FbPage;
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
    /// Facebook Pages managed by this account (platform 3). Optional; absent or
    /// empty ⇒ no pages. Invalid ids are dropped server-side (lenient).
    #[serde(default)]
    pub fb_pages_id: Option<Vec<FbPage>>,
}

fn default_platform_id() -> i32 {
    2 // Default to TikTok
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateSocialAccountDto {
    /// Platform ID (1=reddit, 2=tiktok, 3=facebook, etc.). When provided,
    /// moves the account to a different platform.
    pub platform_id: Option<i32>,
    pub username: Option<String>,
    pub cookie: Option<String>,
    pub proxy_url: Option<String>,
    pub status: Option<String>,
    pub group_id: Option<i32>,
    #[validate(range(min = 1, max = 10000))]
    pub daily_max_replies: Option<i32>,
    pub device_id: Option<String>,
    pub profile_name: Option<String>,
    /// Facebook Pages for this account. Absent ⇒ leave unchanged; `[]` ⇒ clear
    /// all pages (stored as NULL). Invalid ids are dropped server-side.
    #[serde(default)]
    pub fb_pages_id: Option<Vec<FbPage>>,
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
    /// Facebook Pages for this account. Always present; `[]` when none.
    #[serde(default)]
    pub fb_pages_id: Vec<FbPage>,
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
    /// Number of profiles skipped because they already exist for this
    /// (user_id, platform_id)
    #[serde(default)]
    pub skipped_count: i32,
    /// Profile names that were skipped because they already exist
    #[serde(default)]
    pub skipped_profiles: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the "edit platform doesn't persist" bug: the
    /// `PUT /accounts/:id` body carries `platform_id`, and the update DTO must
    /// capture it. Previously the field was absent, so serde silently dropped
    /// it and the platform never changed.
    #[test]
    fn update_account_dto_captures_platform_id() {
        let dto: UpdateSocialAccountDto = serde_json::from_str(
            r#"{
                "platform_id": 3,
                "username": "acct",
                "group_id": 0,
                "device_id": "550e8400-e29b-41d4-a716-446655440000",
                "profile_name": "account_1"
            }"#,
        )
        .expect("valid account update payload");

        assert_eq!(dto.platform_id, Some(3));
    }

    /// When the body omits `platform_id`, the field stays `None` so the service
    /// leaves the account's platform untouched.
    #[test]
    fn update_account_dto_platform_id_is_optional() {
        let dto: UpdateSocialAccountDto =
            serde_json::from_str(r#"{"username": "acct"}"#).expect("valid partial update payload");

        assert_eq!(dto.platform_id, None);
    }

    /// The create body carries `fb_pages_id` as a typed array of `{id,name}`;
    /// serde must capture it (the field-drop class this feature guards against).
    #[test]
    fn create_dto_parses_fb_pages_array() {
        let dto: CreateSocialAccountDto = serde_json::from_str(
            r#"{
                "platform_id": 3,
                "username": "acct",
                "fb_pages_id": [
                    {"id": "100082341853837", "name": "Shop"},
                    {"id": "61556000000000"}
                ]
            }"#,
        )
        .expect("valid create payload with pages");

        let pages = dto.fb_pages_id.expect("fb_pages_id present");
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].id, "100082341853837");
        assert_eq!(pages[0].name.as_deref(), Some("Shop"));
        assert_eq!(pages[1].id, "61556000000000");
        assert_eq!(pages[1].name, None);
    }

    /// The update body carries `fb_pages_id`; absent ⇒ `None` (leave unchanged).
    #[test]
    fn update_dto_parses_fb_pages_array() {
        let with_pages: UpdateSocialAccountDto =
            serde_json::from_str(r#"{"fb_pages_id": [{"id": "100082341853837", "name": "Shop"}]}"#)
                .expect("valid update payload with pages");
        let pages = with_pages.fb_pages_id.expect("fb_pages_id present");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].id, "100082341853837");

        let absent: UpdateSocialAccountDto =
            serde_json::from_str(r#"{"username": "acct"}"#).expect("valid partial update payload");
        assert_eq!(absent.fb_pages_id, None);
    }
}
