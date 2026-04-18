use crate::handler::campaign_handler;
use crate::handler::crawler_handler;
use crate::handler::export_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, patch, post, put},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/", get(campaign_handler::list_campaigns))
        .route("/", post(campaign_handler::create_campaign))
        .route("/:id", get(campaign_handler::get_campaign))
        .route("/:id", put(campaign_handler::update_campaign))
        .route(
            "/:id/status",
            patch(campaign_handler::update_campaign_status),
        )
        .route("/:id/logs", get(campaign_handler::get_campaign_logs))
        .route(
            "/:id/lead-metrics",
            get(campaign_handler::get_campaign_lead_metrics),
        )
        .route(
            "/:id/crawler-tasks",
            get(crawler_handler::list_campaign_tasks),
        )
        // Unified endpoint - returns UnifiedContentDto for all platforms
        .route(
            "/:id/crawler-results",
            get(crawler_handler::list_campaign_results),
        )
        // Alias for crawler-results (for backward compatibility with new endpoints)
        .route(
            "/:id/contents",
            get(crawler_handler::list_campaign_contents_unified),
        )
        .route("/:id/export", get(export_handler::export_campaign_data))
}
