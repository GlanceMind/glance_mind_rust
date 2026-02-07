//! AI Publish Module Handler
//! 自动发布模块 HTTP 处理器

use crate::api_ok;
use crate::dto::aipub_dto::*;
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::state::user_state::UserState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use glance_mind_db::entity::user::User;

// =============================================================================
// Plan Handlers
// =============================================================================

/// POST /publish_plans - Create a new publish plan
pub async fn create_plan(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(payload): ValidatedRequest<CreatePlanDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.aipub_service.create_plan(user.id, payload).await?;
    Ok(api_ok!(result))
}

/// GET /publish_plans - List user's publish plans
pub async fn list_plans(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(query): Query<PlanListQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.aipub_service.list_plans(user.id, query).await?;
    Ok(api_ok!(result))
}

/// GET /publish_plans/:id - Get plan detail
pub async fn get_plan(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
    Query(query): Query<PlanDetailQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .get_plan(user.id, plan_id, query.include)
        .await?;
    Ok(api_ok!(result))
}

/// PUT /publish_plans/:id - Update a plan
pub async fn update_plan(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<UpdatePlanDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .update_plan(user.id, plan_id, payload)
        .await?;
    Ok(api_ok!(result))
}

/// DELETE /publish_plans/:id - Delete a plan
pub async fn delete_plan(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    state.aipub_service.delete_plan(user.id, plan_id).await?;
    Ok(api_ok!(serde_json::json!({"deleted": true})))
}

/// POST /publish_plans/:id/retry - Retry failed plan
pub async fn retry_plan(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
    Json(payload): Json<RetryPlanDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .retry_plan(user.id, plan_id, payload)
        .await?;
    Ok(api_ok!(result))
}

/// GET /publish_plans/:id/ai_tasks - Get AI tasks for a plan
pub async fn get_plan_ai_tasks(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .get_ai_tasks_by_plan(user.id, plan_id)
        .await?;
    Ok(api_ok!(result))
}

/// GET /publish_plans/:id/publish_tasks - Get publish tasks for a plan
pub async fn get_plan_publish_tasks(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(plan_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .get_publish_tasks_by_plan(user.id, plan_id)
        .await?;
    Ok(api_ok!(result))
}

/// GET /publish_tasks - List all user's publish tasks
pub async fn list_publish_tasks(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(query): Query<UserPublishTaskQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .list_user_publish_tasks(user.id, query)
        .await?;
    Ok(api_ok!(result))
}

/// GET /publish_plans/stats - Get plan stats
pub async fn get_plan_stats(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.aipub_service.get_plan_stats(user.id).await?;
    Ok(api_ok!(result))
}

// =============================================================================
// Internal AI Task Handlers (for Scheduler)
// =============================================================================

/// GET /internal/aipub/ai_tasks/processing - Get processing AI tasks
pub async fn get_processing_ai_tasks(
    State(state): State<UserState>,
    Query(query): Query<InternalAiTasksQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .get_processing_ai_tasks(query.limit.unwrap_or(100))
        .await?;
    Ok(api_ok!(result))
}

#[derive(Debug, serde::Deserialize)]
pub struct InternalAiTasksQueryDto {
    pub limit: Option<i64>,
}

/// POST /internal/aipub/ai_tasks/:id/progress - Update AI task progress
pub async fn update_ai_task_progress(
    State(state): State<UserState>,
    Path(task_id): Path<i32>,
    Json(payload): Json<UpdateAiProgressDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .update_ai_progress(task_id, payload.progress)
        .await?;
    Ok(api_ok!(result))
}

/// POST /internal/aipub/ai_tasks/:id/complete - Complete AI task and expand
pub async fn complete_ai_task(
    State(state): State<UserState>,
    Path(task_id): Path<i32>,
    Json(payload): Json<CompleteAiTaskDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .complete_ai_task(task_id, payload.result)
        .await?;
    Ok(api_ok!(result))
}

/// POST /internal/aipub/ai_tasks/:id/fail - Mark AI task as failed
pub async fn fail_ai_task(
    State(state): State<UserState>,
    Path(task_id): Path<i32>,
    Json(payload): Json<FailAiTaskDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .fail_ai_task(task_id, payload.error_message)
        .await?;
    Ok(api_ok!(result))
}

// =============================================================================
// Public Publish Task Handlers (for Executor)
// =============================================================================

/// GET /public/aipub/publish_tasks - Get ready publish tasks for executor
pub async fn get_ready_publish_tasks(
    State(state): State<UserState>,
    Query(query): Query<PublishTaskQueryDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.aipub_service.get_ready_publish_tasks(query).await?;
    Ok(api_ok!(result))
}

/// PATCH /public/aipub/publish_tasks/:id/status - Update publish task status
pub async fn update_publish_task_status(
    State(state): State<UserState>,
    Path(task_id): Path<i32>,
    ValidatedRequest(payload): ValidatedRequest<UpdatePublishTaskStatusDto>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .aipub_service
        .update_publish_task_status(task_id, payload)
        .await?;
    Ok(api_ok!(result))
}

/// POST /public/aipub/publish_tasks/:id/heartbeat - Task heartbeat
pub async fn task_heartbeat(
    State(_state): State<UserState>,
    Path(task_id): Path<i32>,
    Json(_payload): Json<TaskHeartbeatDto>,
) -> Result<impl IntoResponse, ApiError> {
    // For now, just return success. Could be used to track task liveness.
    Ok(api_ok!(TaskHeartbeatResponseDto {
        id: task_id,
        heartbeat_at: chrono::Utc::now(),
    }))
}
