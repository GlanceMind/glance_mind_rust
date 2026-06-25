use crate::handler::feedback_handler;
use crate::state::user_state::UserState;
use axum::{routing::post, Router};

/// Public feedback / contact routes (no auth — usable from public pages).
pub fn routes() -> Router<UserState> {
    Router::new().route("/submit", post(feedback_handler::submit))
}
