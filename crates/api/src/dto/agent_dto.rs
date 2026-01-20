use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentCommentDto {
    pub id: i32,
    pub comment_id: String,
    pub user_nickname: Option<String>,
    pub user_unique_id: Option<String>,
    pub content: Option<String>,
    pub reason: Option<String>,
    pub suggested_reply: Option<String>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
    pub create_time: Option<String>, // ISO 8601 format
    pub digg_count: Option<i32>,     //  From comment data if  available
    pub status: i16,                 // 0=init, 1=processing, 2=completed
}

impl AgentCommentDto {
    pub fn from_entity(comment: glance_mind_db::entity::agent::AgentComment) -> Self {
        Self {
            id: comment.id,
            comment_id: comment.comment_id,
            user_nickname: comment.user_nickname,
            user_unique_id: comment.user_unique_id,
            content: comment.content,
            reason: comment.reason,
            suggested_reply: comment.suggested_reply,
            suggested_dm: comment.suggested_dm,
            suggested_reply_post: comment.suggested_reply_post,
            create_time: comment.create_time.map(|t| t.and_utc().to_rfc3339()),
            digg_count: None, // TODO: Add to entity if needed
            status: comment.status,
        }
    }
}

// Query request for device-based comments
#[derive(Debug, Deserialize)]
pub struct DeviceCommentsQuery {
    pub device_id: String,
    pub status: Option<i16>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

// Response DTO with video_id included
#[derive(Debug, Serialize)]
pub struct CommentWithVideoDto {
    pub id: i32,
    pub comment_id: String,
    pub video_id: String,
    pub content: Option<String>,
    pub status: i16,
    pub user_nickname: Option<String>,
    pub user_unique_id: Option<String>,
    pub suggested_reply: Option<String>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
    pub reason: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub created_at: DateTime<Utc>,
    pub campaign_id: Option<i32>,
    // Campaign auto-interaction settings
    pub auto_like: bool,
    pub auto_follow: bool,
    pub auto_dm: bool,
    pub auto_reply_comments: bool,
    pub auto_reply_post: bool,
    // Randomly selected profile_name from campaign's group
    pub profile_name: Option<String>,
}

// Update comment status request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCommentStatusDto {
    pub comment_id: String,
    #[validate(range(min = 0, max = 2))]
    pub status: i16,
}

// Update response
#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub success: bool,
    pub message: String,
}
