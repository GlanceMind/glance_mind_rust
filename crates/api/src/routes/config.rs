use crate::handler::config_handler;
use crate::state::user_state::UserState;
use axum::{routing::get, Router};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/platforms", get(config_handler::get_platforms))
        .route(
            "/platforms/:platform_id/regions",
            get(config_handler::get_regions_by_platform),
        )
        .route("/ai-models", get(config_handler::get_ai_models))
        .route("/pricing", get(config_handler::get_pricing))
}
