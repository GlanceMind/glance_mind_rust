use crate::handler::social_account_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/", get(social_account_handler::list_accounts))
        .route("/", post(social_account_handler::create_account))
        .route("/batch", post(social_account_handler::batch_create_accounts))
        .route(
            "/statistics",
            get(social_account_handler::get_account_statistics),
        )
        .route("/:id", put(social_account_handler::update_account))
        .route("/:id/verify", post(social_account_handler::verify_account))
        .route(
            "/:id/remove-from-group",
            post(social_account_handler::remove_from_group),
        )
        .route("/:id", delete(social_account_handler::delete_account))
}
