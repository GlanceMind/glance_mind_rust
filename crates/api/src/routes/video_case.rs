use axum::{
    routing::get,
    Router,
};

use crate::handler::video_case_handler;
use crate::state::user_state::UserState;

pub fn video_case_routes() -> Router<UserState> {
    Router::new()
        .route("/", get(video_case_handler::list_video_cases))
        .route("/:id", get(video_case_handler::get_video_case_detail))
        .route("/task/:task_no", get(video_case_handler::get_video_case_by_task_no))
}
