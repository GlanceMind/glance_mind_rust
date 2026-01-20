use crate::handler::{email_verification_handler, public_handler, upload_task_handler};
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn public_routes(user_state: &UserState) -> Router<UserState> {
    // Create email verification sub-router with its own state
    let email_verification_router = Router::new()
        .route(
            "/email-verification/send",
            post(email_verification_handler::send_verification_code),
        )
        .route(
            "/email-verification/verify",
            post(email_verification_handler::verify_code),
        )
        .with_state(Arc::new(user_state.email_verification_service.clone()));

    Router::new()
        .route(
            "/comments/by-device",
            get(public_handler::get_comments_by_device),
        )
        .route(
            "/comments/update-status",
            post(public_handler::update_comment_status),
        )
        .route(
            "/tasks/by-device",
            get(upload_task_handler::get_tasks_by_device),
        )
        .route(
            "/tasks/update-status",
            post(upload_task_handler::update_task_status),
        )
        .merge(email_verification_router)
}
