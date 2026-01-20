use axum::{
    routing::{get, post},
    Router,
};

use crate::handler::video_handler;
use crate::state::user_state::UserState;

pub fn video_routes() -> Router<UserState> {
    Router::new()
        .route("/generate", post(video_handler::create_video))
        .route("/tasks", get(video_handler::get_user_tasks))
        .route("/tasks/:task_id", get(video_handler::get_user_task))
}
