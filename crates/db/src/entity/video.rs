use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::schema::gm_video_generation_tasks;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_video_generation_tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VideoGenerationTask {
    pub id: i32,
    pub user_id: i32,
    pub task_id: String,
    pub generation_id: Option<String>,
    pub prompt: Option<String>,
    pub media_id: Option<String>,
    pub status: String,
    pub progress_pct: Option<BigDecimal>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub provider_post_id: Option<String>,
    pub provider_response: Option<JsonValue>,
    pub cost_points: BigDecimal,
    pub wallet_transaction_id: Option<i32>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub model_id: Option<i32>,
    pub title: Option<String>,
    pub orientation: Option<String>,
    pub video_seconds: Option<String>,
    pub video_size: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_video_generation_tasks)]
pub struct NewVideoGenerationTask {
    pub user_id: i32,
    pub task_id: String,
    pub prompt: Option<String>,
    pub media_id: Option<String>,
    pub status: String,
    pub cost_points: BigDecimal,
    pub wallet_transaction_id: Option<i32>,
    pub model_id: Option<i32>,
    pub title: Option<String>,
    pub orientation: Option<String>,
    pub video_seconds: Option<String>,
    pub video_size: Option<String>,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = gm_video_generation_tasks)]
pub struct UpdateVideoGenerationTask {
    pub status: Option<String>,
    pub progress_pct: Option<BigDecimal>,
    pub generation_id: Option<String>,
    pub video_width: Option<i32>,
    pub video_height: Option<i32>,
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub provider_post_id: Option<String>,
    pub provider_response: Option<JsonValue>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoTaskStatus {
    Pending,
    Queued,
    Processing,
    Succeeded,
    Failed,
    Cancelled,
}

#[allow(dead_code)]
impl VideoTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoTaskStatus::Pending => "pending",
            VideoTaskStatus::Queued => "queued",
            VideoTaskStatus::Processing => "processing",
            VideoTaskStatus::Succeeded => "succeeded",
            VideoTaskStatus::Failed => "failed",
            VideoTaskStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(VideoTaskStatus::Pending),
            "queued" => Some(VideoTaskStatus::Queued),
            "processing" => Some(VideoTaskStatus::Processing),
            "succeeded" => Some(VideoTaskStatus::Succeeded),
            "failed" => Some(VideoTaskStatus::Failed),
            "cancelled" => Some(VideoTaskStatus::Cancelled),
            _ => None,
        }
    }
}

impl std::fmt::Display for VideoTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
