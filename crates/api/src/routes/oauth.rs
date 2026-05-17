use crate::handler::oauth_handler;
use crate::state::oauth_state::OauthState;
use axum::{
    routing::{get, post},
    Router,
};

/// OAuth 2.0 token + revoke routes
/// Mounted at /oauth by root router
pub fn oauth_routes() -> Router<OauthState> {
    Router::new()
        .route("/token", post(oauth_handler::token))
        .route("/revoke", post(oauth_handler::revoke))
}

/// OTA config routes
/// Mounted at /ota by root router
pub fn ota_routes() -> Router<OauthState> {
    Router::new()
        .route("/config", post(oauth_handler::set_ota_config))
        .route("/config/oauth.enabled", get(oauth_handler::get_ota_config))
}
