use crate::handler::dashboard_handler;
use crate::state::user_state::UserState;
use axum::{routing::get, Router};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/overview", get(dashboard_handler::get_overview_stats))
        .route(
            "/performance",
            get(dashboard_handler::get_performance_stats),
        )
        .route(
            "/recent-campaigns",
            get(dashboard_handler::get_recent_campaigns),
        )
}
