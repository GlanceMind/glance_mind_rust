use crate::handler::agent_handler;
use crate::state::user_state::UserState;
use axum::routing::get;
use axum::Router;

pub fn routes(state: UserState) -> Router {
    Router::new()
        .route(
            "/videos/:video_id/comments",
            get(agent_handler::get_video_comments),
        )
        .with_state(state)
}
