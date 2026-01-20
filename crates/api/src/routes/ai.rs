use crate::handler::ai_handler;
use crate::state::user_state::UserState;
use axum::{routing::post, Router};

pub fn routes() -> Router<UserState> {
    Router::new().route("/generate", post(ai_handler::generate_content))
}
