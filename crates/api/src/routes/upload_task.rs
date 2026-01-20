use crate::handler::upload_task_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/", post(upload_task_handler::create_upload_task))
        .route("/my", get(upload_task_handler::list_my_tasks))
}
