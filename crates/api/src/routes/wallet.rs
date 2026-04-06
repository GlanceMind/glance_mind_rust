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
        // XunhuPay recharge endpoints
        .route("/recharge", post(wallet_handler::create_recharge))
        .route(
            "/recharge/:order_no",
            get(wallet_handler::get_recharge_status),
        )
}
