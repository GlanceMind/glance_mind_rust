use crate::error::api_error::ApiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Shared DM types (matching NATS data and frontend types)
// ============================================================

const MAX_REPLY_CONTENT_LEN: usize = 5000;
const VALID_REPLY_MODES: &[&str] = &["manual", "auto"];
const VALID_CONV_STATUSES: &[&str] = &["active", "muted", "archived"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessageDto {
    pub msg_id: String,
    pub conv_id: String,
    pub direction: String,
    pub content: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub platform_msg_id: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    pub timestamp: String,
    #[serde(default)]
    pub nats_seq: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetaDto {
    pub conv_id: String,
    #[serde(default)]
    pub user_id: i32,
    #[serde(default)]
    pub social_account_id: i32,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub platform_id: i32,
    #[serde(default)]
    pub platform_name: String,
    #[serde(default)]
    pub my_username: String,
    #[serde(default)]
    pub my_profile_name: String,
    #[serde(default)]
    pub remote_user_id: String,
    #[serde(default)]
    pub remote_username: String,
    #[serde(default)]
    pub remote_display_name: Option<String>,
    #[serde(default)]
    pub remote_avatar_url: Option<String>,
    #[serde(default)]
    pub last_message_at: String,
    /// Preview text of the last message (alias: "last_message")
    #[serde(default, alias = "last_message")]
    pub last_message_preview: String,
    #[serde(default)]
    pub last_message_direction: String,
    #[serde(default)]
    pub unread_count: i32,
    #[serde(default = "default_active")]
    pub status: String,
    #[serde(default = "default_manual")]
    pub reply_mode: String,
    #[serde(default)]
    pub ai_suggestion: Option<String>,
    #[serde(default)]
    pub updated_at: String,
    /// Username used as display name for sender in the social account context
    #[serde(default)]
    pub social_account_username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusDto {
    pub online: bool,
    pub last_seen: Option<String>,
}

// ============================================================
// API Request DTOs
// ============================================================

#[derive(Debug, Deserialize)]
pub struct DmConversationsQuery {
    pub platform_id: Option<i32>,
    pub device_id: Option<String>,
    pub account_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct DmMessagesQuery {
    pub before_seq: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct DmReplyRequest {
    pub content: String,
    #[serde(default = "default_content_type")]
    pub content_type: String,
}

impl DmReplyRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        let trimmed = self.content.trim();
        if trimmed.is_empty() {
            return Err(ApiError::BadRequest("Reply content must not be empty".into()));
        }
        if trimmed.len() > MAX_REPLY_CONTENT_LEN {
            return Err(ApiError::BadRequest(
                format!("Reply content exceeds {MAX_REPLY_CONTENT_LEN} characters"),
            ));
        }
        Ok(())
    }
}

fn default_content_type() -> String {
    "text".to_string()
}

fn default_active() -> String {
    "active".to_string()
}

fn default_manual() -> String {
    "manual".to_string()
}

#[derive(Debug, Deserialize)]
pub struct DmSettingsRequest {
    pub reply_mode: Option<String>,
    pub status: Option<String>,
}

impl DmSettingsRequest {
    pub fn validate(&self) -> Result<(), ApiError> {
        if let Some(ref mode) = self.reply_mode {
            if !VALID_REPLY_MODES.contains(&mode.as_str()) {
                return Err(ApiError::BadRequest(
                    format!("Invalid reply_mode '{mode}', must be one of: {}", VALID_REPLY_MODES.join(", ")),
                ));
            }
        }
        if let Some(ref status) = self.status {
            if !VALID_CONV_STATUSES.contains(&status.as_str()) {
                return Err(ApiError::BadRequest(
                    format!("Invalid status '{status}', must be one of: {}", VALID_CONV_STATUSES.join(", ")),
                ));
            }
        }
        Ok(())
    }
}

// ============================================================
// API Response DTOs
// ============================================================

#[derive(Debug, Serialize)]
pub struct DmConversationsResponse {
    pub conversations: Vec<ConversationMetaDto>,
    pub device_status: HashMap<String, DeviceStatusDto>,
}

#[derive(Debug, Serialize)]
pub struct DmMessagesResponse {
    pub messages: Vec<DmMessageDto>,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct DmReplyResponse {
    pub cmd_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DmNatsTokenResponse {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct DmStatsResponse {
    pub total_conversations: usize,
    pub total_unread: i32,
    pub per_platform: Vec<DmPlatformStats>,
}

#[derive(Debug, Serialize)]
pub struct DmPlatformStats {
    pub platform_id: i32,
    pub platform_name: String,
    pub conversations: usize,
    pub unread: i32,
}

// ============================================================
// Monitor Config (stored in NATS KV dm_monitor_config)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMonitorConfigDto {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: i32,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_monitors: i32,
    #[serde(default = "default_inbox_linger")]
    pub inbox_linger_seconds: i32,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
}

impl Default for DmMonitorConfigDto {
    fn default() -> Self {
        Self {
            device_id: String::new(),
            enabled: false,
            poll_interval_seconds: default_poll_interval(),
            max_concurrent_monitors: default_max_concurrent(),
            inbox_linger_seconds: default_inbox_linger(),
            platforms: Vec::new(),
            updated_at: String::new(),
        }
    }
}

fn default_poll_interval() -> i32 {
    120
}
fn default_max_concurrent() -> i32 {
    2
}
fn default_inbox_linger() -> i32 {
    30
}

#[derive(Debug, Deserialize)]
pub struct DmMonitorConfigUpdateRequest {
    pub enabled: Option<bool>,
    pub poll_interval_seconds: Option<i32>,
    pub max_concurrent_monitors: Option<i32>,
    pub inbox_linger_seconds: Option<i32>,
    pub platforms: Option<Vec<String>>,
}
