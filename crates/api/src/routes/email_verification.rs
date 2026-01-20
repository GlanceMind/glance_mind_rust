use crate::handler::email_verification_handler;
use crate::service::email_verification_service::EmailVerificationService;
use axum::routing::post;
use axum::Router;
use std::sync::Arc;

#[allow(dead_code)]
pub fn email_verification_routes(service: Arc<EmailVerificationService>) -> Router {
    Router::new()
        .route(
            "/send",
            post(email_verification_handler::send_verification_code),
        )
        .route("/verify", post(email_verification_handler::verify_code))
        // Password reset routes
        .route(
            "/forgot-password",
            post(email_verification_handler::send_password_reset_code),
        )
        .route(
            "/reset-password",
            post(email_verification_handler::reset_password),
        )
        .with_state(service)
}
