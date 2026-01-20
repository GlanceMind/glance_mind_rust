use crate::handler::social_group_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/", get(social_group_handler::list_groups))
        .route("/", post(social_group_handler::create_group))
        .route("/:id", put(social_group_handler::update_group))
        .route("/:id", delete(social_group_handler::delete_group))
}
