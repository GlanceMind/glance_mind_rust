use crate::response::error_code::ErrorCode;
use crate::response::unified_response::ApiResponse;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
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
            TokenError::InvalidToken(_) => ErrorCode::TokenInvalid,
            TokenError::TokenExpired => ErrorCode::TokenExpired,
            TokenError::MissingToken => ErrorCode::Unauthorized,
            TokenError::TokenCreationError(_) => ErrorCode::InternalServerError,
        }
    }

    /// Get Chinese error message
    pub fn to_message_cn(&self) -> String {
        match self {
            TokenError::InvalidToken(msg) => format!("Invalid authentication: {}", msg),
            TokenError::TokenExpired => "Session expired, please login again".to_string(),
            TokenError::MissingToken => "Missing authentication".to_string(),
            TokenError::TokenCreationError(msg) => format!("Token creation error: {}", msg),
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
