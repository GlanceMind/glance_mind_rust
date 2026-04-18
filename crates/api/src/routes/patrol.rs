use crate::handler::patrol_handler;
use crate::state::user_state::UserState;
use axum::{routing::get, Router};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/latest", get(patrol_handler::get_latest))
        .route("/reports", get(patrol_handler::list_reports))
        .route("/reports/:report_id", get(patrol_handler::get_report))
        .route(
            "/accounts/:account_id/trend",
            get(patrol_handler::get_account_trend),
        )
        .route("/summary", get(patrol_handler::get_summary))
}
