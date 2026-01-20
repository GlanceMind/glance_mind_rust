use crate::handler::agent_handler;
use crate::state::user_state::UserState;
use axum::routing::get;
use axum::Router;

pub fn routes(state: UserState) -> Router {
    Router::new()
        // Legacy endpoint - TikTok only
        .route(
            "/videos/:video_id/comments",
            get(agent_handler::get_video_comments),
        )
        // Unified endpoint - all platforms
        // Usage: /agent/contents/:content_db_id/comments?platform_id=1&page=1&page_size=20
        .route(
            "/contents/:content_db_id/comments",
            get(agent_handler::get_unified_comments),
        )
        .with_state(state)
}
