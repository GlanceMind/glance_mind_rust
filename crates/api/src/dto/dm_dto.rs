use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Shared DM types (matching NATS data and frontend types)
// ============================================================

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
