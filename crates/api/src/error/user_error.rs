use crate::response::error_code::ErrorCode;
use crate::response::unified_response::ApiResponse;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Invalid password")]
    InvalidPassword,
}

impl UserError {
    /// Convert UserError to ErrorCode
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            UserError::UserNotFound => ErrorCode::UserNotFound,
            UserError::UserAlreadyExists => ErrorCode::UserAlreadyExists,
            UserError::InvalidPassword => ErrorCode::InvalidPassword,
        }
    }

    /// Get Chinese error message
    pub fn to_message_cn(&self) -> &'static str {
        match self {
            UserError::UserNotFound => "User not found",
            UserError::UserAlreadyExists => "User already exists",
            UserError::InvalidPassword => "Invalid password",
        }
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let error_code = self.to_error_code();
        let msg = self.to_string();
        let msg_cn = self.to_message_cn();

        ApiResponse::<()>::error_with_message(error_code, msg, msg_cn).into_response()
    }
}
