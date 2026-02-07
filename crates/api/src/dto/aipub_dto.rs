//! AI Publish Module DTOs
//! 自动发布模块数据传输对象

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

// =============================================================================
// Plan DTOs
// =============================================================================

/// Create plan request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePlanDto {
    #[validate(length(max = 200))]
    pub name: Option<String>,
    pub group_id: Option<i32>,
    pub social_account_id: Option<i32>,
    #[validate(range(min = 1))]
    pub platform_id: i32,
    #[validate(length(min = 1, max = 20))]
    pub content_type: String,
    /// Plan type: "batch_text" (multiple text posts for group) or "single_video" (one video for account)
    /// Defaults to "batch_text" if not specified
    #[validate(length(max = 20))]
    pub plan_type: Option<String>,
    pub ai_task_types: Option<Vec<String>>,
    pub ai_service_config: Option<JsonValue>,
    pub ai_input: Option<JsonValue>,
    pub content: Option<JsonValue>,
    pub chat_ai_model_id: Option<i32>,
    pub video_ai_model_id: Option<i32>,
    pub image_ai_model_id: Option<i32>,
}

/// Plan response DTO
#[derive(Debug, Clone, Serialize)]
pub struct PlanResponseDto {
    pub id: i32,
    pub name: Option<String>,
    pub user_id: i32,
    pub group_id: Option<i32>,
    pub group_name: Option<String>,
    pub social_account_id: Option<i32>,
    pub account_username: Option<String>,
    pub platform_id: i32,
    pub platform_name: Option<String>,
    pub chat_ai_model_id: Option<i32>,
    pub chat_ai_model_name: Option<String>,
    pub video_ai_model_id: Option<i32>,
    pub video_ai_model_name: Option<String>,
    pub image_ai_model_id: Option<i32>,
    pub image_ai_model_name: Option<String>,
    pub content_type: String,
    /// Plan type: "batch_text", "single_video", or "account_grooming"
    pub plan_type: String,
    pub ai_task_types: Option<Vec<String>>,
    pub ai_service_config: Option<JsonValue>,
    pub ai_input: Option<JsonValue>,
    pub content: Option<JsonValue>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    // Stats
    pub ai_tasks_count: Option<i64>,
    pub publish_tasks_count: Option<i64>,
    pub completed_count: Option<i64>,
    pub failed_count: Option<i64>,
    // Billing
    /// Billing state: "none" / "frozen" / "settled"
    pub billing_status: String,
    /// Total frozen amount
    pub frozen_cost: BigDecimal,
    /// Consumed amount (incremented per sub-task)
    pub consumed_cost: BigDecimal,
}

/// Plan detail response (with tasks)
#[derive(Debug, Clone, Serialize)]
pub struct PlanDetailResponseDto {
    #[serde(flatten)]
    pub plan: PlanResponseDto,
    pub ai_tasks: Option<Vec<AiTaskResponseDto>>,
    pub publish_tasks: Option<Vec<PublishTaskResponseDto>>,
}

/// Plan list query params
#[derive(Debug, Deserialize)]
pub struct PlanListQueryDto {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub status: Option<String>,
    pub platform_id: Option<i32>,
    pub content_type: Option<String>,
    pub plan_type: Option<String>,
}

/// Plan detail query params
#[derive(Debug, Deserialize)]
pub struct PlanDetailQueryDto {
    pub include: Option<String>, // "ai_tasks" | "publish_tasks" | "all"
}

/// Update plan request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePlanDto {
    #[validate(length(max = 200))]
    pub name: Option<String>,
    pub chat_ai_model_id: Option<i32>,
    pub video_ai_model_id: Option<i32>,
    pub image_ai_model_id: Option<i32>,
    pub ai_input: Option<JsonValue>,
}

/// Retry plan request
#[derive(Debug, Deserialize)]
pub struct RetryPlanDto {
    pub retry_scope: String, // "all" | "failed_ai" | "failed_publish"
}

/// Retry plan response
#[derive(Debug, Serialize)]
pub struct RetryPlanResponseDto {
    pub id: i32,
    pub status: String,
    pub retried_ai_tasks: i64,
    pub retried_publish_tasks: i64,
}

impl From<glance_mind_db::entity::aipub::AipubPlan> for PlanResponseDto {
    fn from(plan: glance_mind_db::entity::aipub::AipubPlan) -> Self {
        Self {
            id: plan.id,
            name: plan.name,
            user_id: plan.user_id,
            group_id: plan.group_id,
            group_name: None,
            social_account_id: plan.social_account_id,
            account_username: None,
            platform_id: plan.platform_id,
            platform_name: None,
            chat_ai_model_id: plan.chat_ai_model_id,
            chat_ai_model_name: None,
            video_ai_model_id: plan.video_ai_model_id,
            video_ai_model_name: None,
            image_ai_model_id: plan.image_ai_model_id,
            image_ai_model_name: None,
            content_type: plan.content_type,
            plan_type: plan.plan_type,
            ai_task_types: plan
                .ai_task_types
                .map(|v| v.into_iter().flatten().collect()),
            ai_service_config: plan.ai_service_config,
            ai_input: plan.ai_input,
            content: plan.content,
            status: plan.status,
            created_at: plan.created_at,
            updated_at: plan.updated_at,
            ai_tasks_count: None,
            publish_tasks_count: None,
            completed_count: None,
            failed_count: None,
            billing_status: plan.billing_status,
            frozen_cost: plan.frozen_cost,
            consumed_cost: plan.consumed_cost,
        }
    }
}

// =============================================================================
// AI Task DTOs
// =============================================================================

/// AI Task response DTO
#[derive(Debug, Clone, Serialize)]
pub struct AiTaskResponseDto {
    pub id: i32,
    pub plan_id: i32,
    pub task_type: String,
    pub external_service: String,
    pub external_job_id: Option<String>,
    pub input: JsonValue,
    pub result: Option<JsonValue>,
    pub status: String,
    pub progress: i32,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<glance_mind_db::entity::aipub::AipubAiTask> for AiTaskResponseDto {
    fn from(task: glance_mind_db::entity::aipub::AipubAiTask) -> Self {
        Self {
            id: task.id,
            plan_id: task.plan_id,
            task_type: task.task_type,
            external_service: task.external_service,
            external_job_id: task.external_job_id,
            input: task.input,
            result: task.result,
            status: task.status,
            progress: task.progress.unwrap_or(0),
            error_message: task.error_message,
            retry_count: task.retry_count.unwrap_or(0),
            created_at: task.created_at,
            updated_at: task.updated_at,
            completed_at: task.completed_at,
        }
    }
}

/// Internal: Update AI task progress request
#[derive(Debug, Deserialize)]
pub struct UpdateAiProgressDto {
    pub progress: i32,
}

/// Internal: Complete AI task request
#[derive(Debug, Deserialize)]
pub struct CompleteAiTaskDto {
    pub result: JsonValue,
}

/// Internal: Fail AI task request
#[derive(Debug, Deserialize)]
pub struct FailAiTaskDto {
    pub error_message: String,
}

/// Internal: Complete AI task response
#[derive(Debug, Serialize)]
pub struct CompleteAiTaskResponseDto {
    pub id: i32,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub expanded_tasks_count: i64,
}

// =============================================================================
// Publish Task DTOs
// =============================================================================

/// Publish Task response DTO
#[derive(Debug, Clone, Serialize)]
pub struct PublishTaskResponseDto {
    pub id: i32,
    pub plan_id: i32,
    pub social_account_id: i32,
    pub account_username: Option<String>,
    pub profile_name: Option<String>,
    pub platform: Option<String>,
    pub platform_id: Option<i32>,
    pub content_type: Option<String>,
    pub content: JsonValue,
    pub status: String,
    pub result_url: Option<String>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<glance_mind_db::entity::aipub::AipubTask> for PublishTaskResponseDto {
    fn from(task: glance_mind_db::entity::aipub::AipubTask) -> Self {
        Self {
            id: task.id,
            plan_id: task.plan_id,
            social_account_id: task.social_account_id,
            account_username: None,
            profile_name: None,
            platform: None,
            platform_id: None,
            content_type: None,
            content: task.content,
            status: task.status,
            result_url: task.result_url,
            error_message: task.error_message,
            retry_count: task.retry_count.unwrap_or(0),
            created_at: task.created_at,
            updated_at: task.updated_at,
            published_at: task.published_at,
        }
    }
}

/// User: Query publish tasks list
#[derive(Debug, Deserialize)]
pub struct UserPublishTaskQueryDto {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub status: Option<String>,
    pub platform_id: Option<i32>,
}

/// Public: Query publish tasks (for executor)
#[derive(Debug, Deserialize)]
pub struct PublishTaskQueryDto {
    pub device_id: String,
    pub status: Option<String>,
    pub platform: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

/// Public: Update publish task status request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePublishTaskStatusDto {
    #[validate(length(min = 1, max = 20))]
    pub status: String,
    pub result_url: Option<String>,
    pub error_message: Option<String>,
    pub execution_log: Option<String>,
}

/// Public: Task heartbeat request
#[derive(Debug, Deserialize)]
pub struct TaskHeartbeatDto {
    pub progress: Option<i32>,
    pub message: Option<String>,
}

/// Public: Task heartbeat response
#[derive(Debug, Serialize)]
pub struct TaskHeartbeatResponseDto {
    pub id: i32,
    pub heartbeat_at: DateTime<Utc>,
}

/// Executor publish task (for executor to pull)
#[derive(Debug, Clone, Serialize)]
pub struct ExecutorPublishTaskDto {
    pub task_id: i32,
    pub plan_id: i32,
    pub social_account_id: i32,
    pub platform: String,
    pub platform_id: i32,
    pub content_type: String,
    pub plan_type: String,
    pub profile_name: String,
    pub content: JsonValue,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Stats DTOs
// =============================================================================

/// Plan stats response
#[derive(Debug, Serialize)]
pub struct PlanStatsDto {
    pub total_plans: i64,
    pub ai_processing: i64,
    pub ready: i64,
    pub completed: i64,
    pub failed: i64,
}
