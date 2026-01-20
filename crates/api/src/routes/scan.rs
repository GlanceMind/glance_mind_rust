use crate::handler::scan_handler;
use crate::state::user_state::UserState;
use axum::{routing::post, Router};

pub fn routes() -> Router<UserState> {
    Router::new().route("/post", post(scan_handler::scan_post))
}
