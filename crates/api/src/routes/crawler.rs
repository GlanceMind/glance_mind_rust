use crate::handler::crawler_handler;
use crate::state::user_state::UserState;
use axum::{routing::get, Router};

pub fn routes() -> Router<UserState> {
    Router::new().route("/:id/results", get(crawler_handler::list_task_results))
}
