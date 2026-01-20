use crate::handler::wallet_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/balance", get(wallet_handler::get_wallet_balance))
        .route(
            "/transactions",
            get(wallet_handler::get_wallet_transactions),
        )
        .route("/orders", post(wallet_handler::create_recharge_order))
        .route("/orders/:order_id", get(wallet_handler::get_order_status))
}
