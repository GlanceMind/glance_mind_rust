use crate::handler::user_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/me", get(user_handler::me))
        .route("/profile", post(user_handler::update_profile))
        .route("/password", post(user_handler::change_password))
        .route("/api-key", post(user_handler::regenerate_api_key))
        .route("/referrals", get(user_handler::get_referral_stats))
}
