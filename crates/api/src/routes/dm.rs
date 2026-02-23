use crate::handler::dm_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post, put},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/conversations", get(dm_handler::list_conversations))
        .route(
            "/conversations/:conv_id/messages",
            get(dm_handler::get_messages),
        )
        .route(
            "/conversations/:conv_id/reply",
            post(dm_handler::send_reply),
        )
        .route(
            "/conversations/:conv_id/read",
            post(dm_handler::mark_read),
        )
        .route(
            "/conversations/:conv_id/settings",
            put(dm_handler::update_settings),
        )
        .route("/stats", get(dm_handler::get_stats))
        .route("/nats-token", get(dm_handler::get_nats_token))
        .route(
            "/monitor-config/:device_id",
            get(dm_handler::get_monitor_config).put(dm_handler::update_monitor_config),
        )
}
