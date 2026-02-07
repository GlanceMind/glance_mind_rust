//! AI Publish Module Entities
//! 自动发布模块实体定义

use crate::schema::{gm_aipub_ai_tasks, gm_aipub_plans, gm_aipub_tasks};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// =============================================================================
// Plan Entity - 发布计划
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_aipub_plans)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AipubPlan {
    pub id: i32,
    pub user_id: i32,
    pub group_id: Option<i32>,
    pub social_account_id: Option<i32>,
    pub platform_id: i32,
    pub content_type: String,
    pub ai_task_types: Option<Vec<Option<String>>>,
    pub ai_service_config: Option<JsonValue>,
    pub ai_input: Option<JsonValue>,
    pub content: Option<JsonValue>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub chat_ai_model_id: Option<i32>,
    pub video_ai_model_id: Option<i32>,
    pub name: Option<String>,
    /// Plan type: batch_text, single_video, or account_grooming
    pub plan_type: String,
    pub image_ai_model_id: Option<i32>,
    /// Billing state: none=no billing, frozen=budget frozen, settled=finalized
    pub billing_status: String,
    /// Total frozen amount (set at creation, immutable)
    pub frozen_cost: BigDecimal,
    /// Consumed amount (incremented per sub-task completion)
    pub consumed_cost: BigDecimal,
    /// Freeze timestamp for reconciliation timeout detection
    pub frozen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_aipub_plans)]
pub struct NewAipubPlan {
    pub user_id: i32,
    pub name: Option<String>,
    pub group_id: Option<i32>,
    pub social_account_id: Option<i32>,
    pub platform_id: i32,
    pub content_type: String,
    pub ai_task_types: Option<Vec<Option<String>>>,
    pub ai_service_config: Option<JsonValue>,
    pub ai_input: Option<JsonValue>,
    pub content: Option<JsonValue>,
    pub status: String,
    pub chat_ai_model_id: Option<i32>,
    pub video_ai_model_id: Option<i32>,
    /// Plan type: batch_text, single_video, or account_grooming
    pub plan_type: String,
    pub image_ai_model_id: Option<i32>,
}

#[derive(Debug, Clone, Default, AsChangeset)]
#[diesel(table_name = gm_aipub_plans)]
pub struct UpdateAipubPlan {
    pub name: Option<String>,
    pub status: Option<String>,
    pub content: Option<JsonValue>,
    pub ai_input: Option<JsonValue>,
    pub chat_ai_model_id: Option<Option<i32>>,
    pub video_ai_model_id: Option<Option<i32>>,
    pub image_ai_model_id: Option<Option<i32>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Plan Status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Pending,
    AiProcessing,
    Ready,
    Completed,
    Failed,
}

impl PlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanStatus::Pending => "pending",
            PlanStatus::AiProcessing => "ai_processing",
            PlanStatus::Ready => "ready",
            PlanStatus::Completed => "completed",
            PlanStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(PlanStatus::Pending),
            "ai_processing" => Some(PlanStatus::AiProcessing),
            "ready" => Some(PlanStatus::Ready),
            "completed" => Some(PlanStatus::Completed),
            "failed" => Some(PlanStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Plan Type enum - distinguishes between batch text and single video plans
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PlanType {
    /// Batch text generation for a group (multiple accounts, text posts only)
    #[default]
    BatchText,
    /// Single video generation for one account (video + content)
    SingleVideo,
    /// Account grooming: generate profile names + avatars for a group
    AccountGrooming,
}

impl PlanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::BatchText => "batch_text",
            PlanType::SingleVideo => "single_video",
            PlanType::AccountGrooming => "account_grooming",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "batch_text" => Some(PlanType::BatchText),
            "single_video" => Some(PlanType::SingleVideo),
            "account_grooming" => Some(PlanType::AccountGrooming),
            _ => None,
        }
    }

    /// Returns true if this plan type requires video generation
    pub fn needs_video(&self) -> bool {
        matches!(self, PlanType::SingleVideo)
    }

    /// Returns true if this plan type targets a group (multiple accounts)
    pub fn targets_group(&self) -> bool {
        matches!(self, PlanType::BatchText | PlanType::AccountGrooming)
    }
}


impl std::fmt::Display for PlanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// AI Task Entity - AI 生成任务
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_aipub_ai_tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AipubAiTask {
    pub id: i32,
    pub plan_id: i32,
    pub task_type: String,
    pub external_service: String,
    pub external_job_id: Option<String>,
    pub input: JsonValue,
    pub result: Option<JsonValue>,
    pub status: String,
    pub progress: Option<i32>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Execution order within the plan (0=first)
    pub sequence: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_aipub_ai_tasks)]
pub struct NewAipubAiTask {
    pub plan_id: i32,
    pub task_type: String,
    pub external_service: String,
    pub external_job_id: Option<String>,
    pub input: JsonValue,
    pub status: String,
}

#[derive(Debug, Clone, Default, AsChangeset)]
#[diesel(table_name = gm_aipub_ai_tasks)]
pub struct UpdateAipubAiTask {
    pub external_job_id: Option<String>,
    pub result: Option<JsonValue>,
    pub status: Option<String>,
    pub progress: Option<i32>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// AI Task Type enum
/// DB CHECK: ('video_gen','content_gen','image_gen','combined','account_grooming')
/// Source of truth: aipub.proto AiTaskType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTaskType {
    ContentGen,
    VideoGen,
    ImageGen,
    /// Content + Video combined generation
    Combined,
    /// Account grooming: generate name + avatar for profile
    AccountGrooming,
}

impl AiTaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiTaskType::ContentGen => "content_gen",
            AiTaskType::VideoGen => "video_gen",
            AiTaskType::ImageGen => "image_gen",
            AiTaskType::Combined => "combined",
            AiTaskType::AccountGrooming => "account_grooming",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "content_gen" => Some(AiTaskType::ContentGen),
            "video_gen" => Some(AiTaskType::VideoGen),
            "image_gen" => Some(AiTaskType::ImageGen),
            "combined" => Some(AiTaskType::Combined),
            "account_grooming" => Some(AiTaskType::AccountGrooming),
            _ => None,
        }
    }
}

impl std::fmt::Display for AiTaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// AI Task Status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTaskStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl AiTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiTaskStatus::Pending => "pending",
            AiTaskStatus::Processing => "processing",
            AiTaskStatus::Completed => "completed",
            AiTaskStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AiTaskStatus::Pending),
            "processing" => Some(AiTaskStatus::Processing),
            "completed" => Some(AiTaskStatus::Completed),
            "failed" => Some(AiTaskStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for AiTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// Publish Task Entity - 发布任务 (每个账户一条)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_aipub_tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AipubTask {
    pub id: i32,
    pub plan_id: i32,
    pub social_account_id: i32,
    pub content: JsonValue,
    pub status: String,
    pub result_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    /// Reference to the AI task that generated this publish task content
    pub ai_task_id: Option<i32>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_aipub_tasks)]
pub struct NewAipubTask {
    pub plan_id: i32,
    pub social_account_id: i32,
    pub content: JsonValue,
    pub status: String,
    /// Reference to the AI task that generated this publish task content
    pub ai_task_id: Option<i32>,
}

#[derive(Debug, Clone, Default, AsChangeset)]
#[diesel(table_name = gm_aipub_tasks)]
pub struct UpdateAipubTask {
    pub status: Option<String>,
    pub result_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: Option<i32>,
    pub updated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
}

/// Publish Task Status enum
/// DB CHECK: ('pending','video_pending','video_processing','ready',
///            'processing','completed','failed')
/// Source of truth: aipub.proto PublishTaskStatus
/// Includes historical video-pipeline states for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishTaskStatus {
    Pending,
    /// Historical: waiting for video generation
    VideoPending,
    /// Historical: video being generated
    VideoProcessing,
    /// Ready for executor to pick up
    Ready,
    /// Executor is processing
    Processing,
    /// Published successfully
    Completed,
    Failed,
}

impl PublishTaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PublishTaskStatus::Pending => "pending",
            PublishTaskStatus::VideoPending => "video_pending",
            PublishTaskStatus::VideoProcessing => "video_processing",
            PublishTaskStatus::Ready => "ready",
            PublishTaskStatus::Processing => "processing",
            PublishTaskStatus::Completed => "completed",
            PublishTaskStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(PublishTaskStatus::Pending),
            "video_pending" => Some(PublishTaskStatus::VideoPending),
            "video_processing" => Some(PublishTaskStatus::VideoProcessing),
            "ready" => Some(PublishTaskStatus::Ready),
            "processing" => Some(PublishTaskStatus::Processing),
            "completed" => Some(PublishTaskStatus::Completed),
            "failed" => Some(PublishTaskStatus::Failed),
            _ => None,
        }
    }
}

impl std::fmt::Display for PublishTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Content Type enum
/// DB CHECK: ('post', 'video', 'reel', 'story', 'profile')
/// Source of truth: aipub.proto ContentType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Post,
    Video,
    Reel,
    Story,
    /// For account grooming profile updates
    Profile,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Post => "post",
            ContentType::Video => "video",
            ContentType::Reel => "reel",
            ContentType::Story => "story",
            ContentType::Profile => "profile",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "post" => Some(ContentType::Post),
            "video" => Some(ContentType::Video),
            "reel" => Some(ContentType::Reel),
            "story" => Some(ContentType::Story),
            "profile" => Some(ContentType::Profile),
            _ => None,
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
