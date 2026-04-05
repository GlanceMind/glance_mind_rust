//! AI Publish Module Routes
//! 自动发布模块路由定义

use crate::handler::{aipub_handler, oss_handler};
use crate::state::user_state::UserState;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, patch, post},
    Router,
};

/// User-facing API routes (requires JWT auth)
/// Auth middleware will be applied by root.rs when merging
pub fn aipub_user_routes() -> Router<UserState> {
    // Body size limit: 100MB for video uploads
    const MAX_VIDEO_UPLOAD_SIZE: usize = 100 * 1024 * 1024; // 100MB

    Router::new()
        // Stats & estimate (put before :id routes to avoid conflict)
        .route("/publish_plans/stats", get(aipub_handler::get_plan_stats))
        .route("/publish_plans/estimate", post(aipub_handler::estimate_plan_cost))
        // Image upload to OSS
        .route("/aipub/upload-image", post(oss_handler::upload_image))
        // Video upload to OSS - with increased body size limit
        .route(
            "/oss/upload-video",
            post(oss_handler::upload_video).layer(DefaultBodyLimit::max(MAX_VIDEO_UPLOAD_SIZE)),
        )
        // Plan CRUD
        .route(
            "/publish_plans",
            post(aipub_handler::create_plan).get(aipub_handler::list_plans),
        )
        .route(
            "/publish_plans/:id",
            get(aipub_handler::get_plan)
                .put(aipub_handler::update_plan)
                .delete(aipub_handler::delete_plan),
        )
        .route("/publish_plans/:id/retry", post(aipub_handler::retry_plan))
        // Plan sub-resources
        .route(
            "/publish_plans/:id/ai_tasks",
            get(aipub_handler::get_plan_ai_tasks),
        )
        .route(
            "/publish_plans/:id/publish_tasks",
            get(aipub_handler::get_plan_publish_tasks),
        )
        // Publish Tasks (all user's tasks)
        .route("/publish_tasks", get(aipub_handler::list_publish_tasks))
}

/// Internal API routes (for Scheduler, requires internal API key)
pub fn aipub_internal_routes() -> Router<UserState> {
    Router::new()
        .route(
            "/internal/aipub/ai_tasks/processing",
            get(aipub_handler::get_processing_ai_tasks),
        )
        .route(
            "/internal/aipub/ai_tasks/:id/progress",
            post(aipub_handler::update_ai_task_progress),
        )
        .route(
            "/internal/aipub/ai_tasks/:id/complete",
            post(aipub_handler::complete_ai_task),
        )
        .route(
            "/internal/aipub/ai_tasks/:id/fail",
            post(aipub_handler::fail_ai_task),
        )
}

/// Public API routes (for Executor, requires device token)
pub fn aipub_public_routes() -> Router<UserState> {
    Router::new()
        .route(
            "/public/aipub/publish_tasks",
            get(aipub_handler::get_ready_publish_tasks),
        )
        .route(
            "/public/aipub/publish_tasks/:id/status",
            patch(aipub_handler::update_publish_task_status),
        )
        .route(
            "/public/aipub/publish_tasks/:id/heartbeat",
            post(aipub_handler::task_heartbeat),
        )
}
