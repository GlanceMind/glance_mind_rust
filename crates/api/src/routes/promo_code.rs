use crate::handler::promo_code_handler;
use crate::service::promo_code_service::PromoCodeService;
use axum::routing::post;
use axum::Router;
use std::sync::Arc;

pub fn promo_code_routes(service: Arc<PromoCodeService>) -> Router {
    Router::new()
        .route("/redeem", post(promo_code_handler::redeem_promo_code))
        .with_state(service)
}
