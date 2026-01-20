use crate::handler::agent_analysis_handler;
use crate::state::user_state::UserState;
use axum::{routing::post, Router};

pub fn routes() -> Router<UserState> {
    Router::new().route("/analyze", post(agent_analysis_handler::analyze_comments))
}
