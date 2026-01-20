use crate::handler::auth_handler;
use crate::state::auth_state::AuthState;
use axum::{routing::post, Router};

pub fn routes() -> Router<AuthState> {
    Router::new()
        .route("/login", post(auth_handler::auth))
        .route("/auth", post(auth_handler::auth)) // Compatible with legacy path
        .route("/google", post(auth_handler::google_auth)) // Google OAuth2 login
}
