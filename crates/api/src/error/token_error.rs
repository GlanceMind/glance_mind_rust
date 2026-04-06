use crate::response::error_code::ErrorCode;
use crate::response::unified_response::ApiResponse;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token has expired")]
    TokenExpired,
    #[error("Missing Bearer token")]
    MissingToken,
    #[error("Token error: {0}")]
    TokenCreationError(String),
}

impl TokenError {
    /// Convert TokenError to ErrorCode
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            TokenError::InvalidToken => ErrorCode::TokenInvalid,
            TokenError::TokenExpired => ErrorCode::TokenExpired,
            TokenError::MissingToken => ErrorCode::Unauthorized,
            TokenError::TokenCreationError(_) => ErrorCode::InternalServerError,
        }
    }

    /// Get Chinese error message
    pub fn to_message_cn(&self) -> String {
        match self {
            TokenError::InvalidToken => "Invalid authentication token".to_string(),
            TokenError::TokenExpired => "Session expired, please login again".to_string(),
            TokenError::MissingToken => "Missing authentication".to_string(),
            TokenError::TokenCreationError(_) => "Authentication service error".to_string(),
        }
    }
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let error_code = self.to_error_code();
        let msg = self.to_string();
        let msg_cn = self.to_message_cn();

        ApiResponse::<()>::error_with_message(error_code, msg, msg_cn).into_response()
    }
}
